mod tools;
mod transport;

use crate::{
    app::Editor,
    editor_operations::{OperationError, OperationReply, RemoteRequest},
};
use eframe::egui;
use rmcp::ServiceExt;
use rmcp::model::RequestId;
use std::{io, sync::Arc, thread, time::Duration};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use transport::{BoundedTransport, Shared};

const UI_QUEUE_CAPACITY: usize = 16;
const UI_REQUESTS_PER_FRAME: usize = 8;
const UI_REPLY_TIMEOUT: Duration = Duration::from_secs(5);
const RUNTIME_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(1);

struct EditorEnvelope {
    request: RemoteRequest,
    reply: oneshot::Sender<Result<OperationReply, OperationError>>,
    cancellation: CancellationToken,
    request_id: RequestId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CloseDelivery {
    Waiting,
    Flushed,
    Failed,
}

pub(crate) struct UiBridge {
    receiver: mpsc::Receiver<EditorEnvelope>,
    shared: Arc<Shared>,
    stop: CancellationToken,
    thread: Option<thread::JoinHandle<()>>,
    close_requested: bool,
}

impl UiBridge {
    pub(crate) fn poll_close_delivery(&mut self) -> Option<CloseDelivery> {
        let delivery = self.shared.poll_close_delivery();
        if delivery == Some(CloseDelivery::Failed) {
            self.close_requested = true;
        }
        delivery
    }
    #[cfg(test)]
    pub(crate) fn test_read_observed(&self) -> bool {
        self.shared.read_observed()
    }

    #[cfg(test)]
    pub(crate) fn test_queue_len(&self) -> usize {
        self.receiver.len()
    }

    #[cfg(test)]
    pub(crate) fn test_alive(&self) -> bool {
        self.shared.alive()
    }
    pub(crate) fn start(context: egui::Context) -> io::Result<Self> {
        let (sender, receiver) = mpsc::channel(UI_QUEUE_CAPACITY);
        let shared = Arc::new(Shared::new(context));
        let stop = CancellationToken::new();
        let thread_shared = shared.clone();
        let thread_stop = stop.clone();
        let thread = thread::Builder::new()
            .name("azteccraft-mcp".into())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        eprintln!("MCP runtime startup failed: {error}");
                        thread_shared.close();
                        return;
                    }
                };
                runtime.block_on(async {
                    let server = tools::EditorServer::new(sender, thread_shared.clone());
                    let transport = BoundedTransport::new(
                        tokio::io::stdin(),
                        tokio::io::stdout(),
                        thread_shared.clone(),
                    );
                    let serving = async {
                        match server.serve(transport).await {
                            Ok(service) => {
                                if service.waiting().await.is_err() {
                                    eprintln!("MCP service task failed");
                                }
                            }
                            Err(_) => eprintln!("MCP initialization failed or connection closed"),
                        }
                    };
                    tokio::select! {
                        _ = thread_stop.cancelled() => (),
                        _ = serving => (),
                    }
                });
                thread_shared.close();
                runtime.shutdown_timeout(RUNTIME_SHUTDOWN_TIMEOUT);
            })?;
        Ok(Self {
            receiver,
            shared,
            stop,
            thread: Some(thread),
            close_requested: false,
        })
    }

    pub(crate) fn take_close_request(&mut self) -> bool {
        if !self.shared.alive() && !self.close_requested {
            self.close_requested = true;
            true
        } else {
            false
        }
    }

    pub(crate) fn dispatch(&mut self, editor: &mut Editor, context: &egui::Context) {
        for _ in 0..UI_REQUESTS_PER_FRAME {
            if !self.shared.alive() {
                break;
            }
            let Ok(envelope) = self.receiver.try_recv() else {
                break;
            };
            if envelope.reply.is_closed()
                || envelope.cancellation.is_cancelled()
                || !self.shared.alive()
            {
                continue;
            }
            let result = editor.remote_operation(envelope.request, context);
            let changed = matches!(
                &result,
                Ok(OperationReply::Created { .. }
                    | OperationReply::Changed
                    | OperationReply::DocumentApplied { .. }
                    | OperationReply::PendingConfirmation { .. }
                    | OperationReply::Cancelled { .. }
                    | OperationReply::CloseRequested { .. })
            );
            if matches!(&result, Ok(OperationReply::CloseRequested { .. })) {
                self.shared
                    .arm_close(envelope.request_id.clone(), &envelope.reply);
            }
            if envelope.reply.send(result).is_err() {
                self.shared.fail_close(&envelope.request_id);
            }
            if changed {
                context.request_repaint();
            }
        }
        if self.shared.alive() && !self.receiver.is_empty() {
            context.request_repaint();
        }
    }
}

impl Drop for UiBridge {
    fn drop(&mut self) {
        self.shared.close();
        self.stop.cancel();
        if let Some(thread) = self.thread.take()
            && thread.is_finished()
            && thread.join().is_err()
        {
            eprintln!("MCP runtime thread failed");
        }
    }
}

#[cfg(test)]
pub(crate) struct TestIngress {
    sender: mpsc::Sender<EditorEnvelope>,
    shared: Arc<Shared>,
}

#[cfg(test)]
impl TestIngress {
    pub(crate) fn queue(
        &self,
        request: RemoteRequest,
    ) -> Result<oneshot::Receiver<Result<OperationReply, OperationError>>, ()> {
        let (reply, receiver) = oneshot::channel();
        self.sender
            .try_send(EditorEnvelope {
                request,
                reply,
                cancellation: CancellationToken::new(),
                request_id: RequestId::Number(-1),
            })
            .map_err(|_| ())?;
        Ok(receiver)
    }

    pub(crate) fn close(&self) {
        self.shared.close();
    }
}

#[cfg(test)]
pub(crate) fn test_bridge(context: egui::Context) -> (UiBridge, TestIngress) {
    let (sender, receiver) = mpsc::channel(UI_QUEUE_CAPACITY);
    let shared = Arc::new(Shared::new(context));
    let bridge = UiBridge {
        receiver,
        shared: shared.clone(),
        stop: CancellationToken::new(),
        thread: None,
        close_requested: false,
    };
    (bridge, TestIngress { sender, shared })
}

#[cfg(all(test, feature = "mcp-test-client"))]
pub(crate) fn test_connection(
    context: egui::Context,
) -> (
    UiBridge,
    tokio::io::DuplexStream,
    impl Future<Output = ()> + Send,
) {
    let (bridge, ingress) = test_bridge(context);
    let (server_io, client_io) = tokio::io::duplex(transport::MAX_EGRESS_BYTES);
    let (reader, writer) = tokio::io::split(server_io);
    let server = tools::EditorServer::new(ingress.sender, ingress.shared.clone());
    let transport = BoundedTransport::new(reader, writer, ingress.shared);
    let serving = async {
        server
            .serve(transport)
            .await
            .unwrap()
            .waiting()
            .await
            .unwrap();
    };
    (bridge, client_io, serving)
}
