use crate::editor_operations::{
    ActiveTool, Operation, OperationError, OperationReply, SelectionMode,
};
#[cfg(any(test, feature = "mcp"))]
use crate::editor_operations::{
    CameraInspection, FaceSelectionPolicy, FaceSelectionTarget, NodeInspection, RemoteRequest,
    SceneInspection, ShapeInspection, TransformInput, UnitsInspection,
};
#[cfg(any(test, feature = "mcp"))]
use azteccraft::native_scene::{LinearUnit, NativeTransform};
use azteccraft::{
    history::{Command, EditOutcome, History, MoveProvenance},
    native_scene::{AngularUnit, FaceTarget, NativeId},
    scene::{NodeId, Primitive},
    view_scene::ViewScene,
    viewport::Camera,
};
use eframe::egui::{self, Color32, Key, RichText};
#[cfg(test)]
use glam::Vec3;
use uuid::Uuid;

mod dispatch;
mod features;
#[path = "../file_ui.rs"]
mod files;
mod inspect;
mod menus;
mod panels;
mod shortcuts;

struct MaterialDraft {
    scene_token: Uuid,
    shape: NativeId,
    original: f64,
    text: String,
    intent: bool,
    focus: egui::Id,
}

impl MaterialDraft {
    fn changed(&self) -> bool {
        self.intent || self.text != self.original.to_string()
    }
}

#[derive(Clone, Copy)]
enum SelectionModeIntent {
    Toggle,
    Face,
}

pub struct Editor {
    #[cfg(feature = "mcp")]
    mcp: Option<crate::mcp::UiBridge>,
    scene_token: Uuid,
    history: History,
    selected: Option<NodeId>,
    selected_path: Vec<NativeId>,
    selection_mode: SelectionMode,
    remembered_face: Option<FaceTarget>,
    camera: Camera,
    grid_visible: bool,
    active_tool: ActiveTool,
    native_move: Option<features::move_tool::NativeMove>,
    move_viewport: Option<features::move_tool::MoveViewportFeedback>,
    move_consumed_frame: bool,
    move_suppress_primary: bool,
    renderer: Option<crate::viewport_renderer::ViewportRenderer>,
    viewport_error: Option<crate::viewport_renderer::ViewportError>,
    name: String,
    channels: [String; 9],
    channel_intents: [bool; 9],
    name_intent: bool,
    material_shape: Option<NativeId>,
    material_draft: Option<MaterialDraft>,
    native_input_pending: bool,
    status: String,
    error: bool,
    aspect: f32,
    files: files::FileState,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            #[cfg(feature = "mcp")]
            mcp: None,
            scene_token: Uuid::new_v4(),
            history: History::default(),
            selected: None,
            selected_path: Vec::new(),
            selection_mode: SelectionMode::default(),
            remembered_face: None,
            camera: Camera::default(),
            grid_visible: true,
            active_tool: ActiveTool::Select,
            native_move: None,
            move_viewport: None,
            move_consumed_frame: false,
            move_suppress_primary: false,
            renderer: None,
            viewport_error: None,
            name: String::new(),
            channels: Default::default(),
            channel_intents: [false; 9],
            name_intent: false,
            material_shape: None,
            material_draft: None,
            native_input_pending: false,
            status: "Ready | Create a cube or plane to start".into(),
            error: false,
            aspect: 1.0,
            files: files::FileState::default(),
        }
    }
}

impl Editor {
    #[cfg(feature = "mcp")]
    pub(crate) fn start_mcp(&mut self, context: &egui::Context) -> std::io::Result<()> {
        self.mcp = Some(crate::mcp::UiBridge::start(context.clone())?);
        Ok(())
    }
    pub fn new(
        cc: &eframe::CreationContext<'_>,
    ) -> Result<Self, crate::viewport_renderer::RenderError> {
        let renderer = crate::viewport_renderer::ViewportRenderer::initialize(cc)?;
        let mut style = egui::Style {
            visuals: egui::Visuals::dark(),
            ..Default::default()
        };
        style.visuals.panel_fill = Color32::from_gray(48);
        style.visuals.window_fill = Color32::from_gray(52);
        style.visuals.selection.bg_fill = Color32::from_rgb(58, 92, 120);
        style.spacing.item_spacing = egui::vec2(6.0, 5.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        cc.egui_ctx.set_style_of(egui::Theme::Dark, style);
        Ok(Self {
            renderer: Some(renderer),
            ..Self::default()
        })
    }
}

impl eframe::App for Editor {
    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        let pointer_held = ctx.input(|input| input.pointer.any_down())
            || input
                .events
                .iter()
                .any(|event| matches!(event, egui::Event::PointerButton { pressed: true, .. }));
        self.native_input_pending |= input.events.iter().any(|event| match event {
            egui::Event::PointerMoved(_) | egui::Event::MouseMoved(_) => pointer_held,
            egui::Event::Key { pressed, .. } => *pressed,
            egui::Event::Copy
            | egui::Event::Screenshot { .. }
            | egui::Event::ModifiersChanged(_)
            | egui::Event::WindowFocused(_)
            | egui::Event::PointerGone => false,
            egui::Event::Cut
            | egui::Event::Paste(_)
            | egui::Event::Text(_)
            | egui::Event::PointerButton { .. }
            | egui::Event::Zoom(_)
            | egui::Event::Rotate(_)
            | egui::Event::Ime(_)
            | egui::Event::Touch { .. }
            | egui::Event::MouseWheel { .. }
            | egui::Event::AccessKitActionRequest(_) => true,
        });
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|input| input.viewport().visible()) == Some(false) {
            self.move_viewport = None;
            if self
                .history
                .active_move()
                .is_some_and(|active| active.provenance == MoveProvenance::Native)
                && let Err(error) = self.cancel_move()
            {
                self.status = error.to_string();
                self.error = true;
            }
            self.handle_close(ctx);
            #[cfg(feature = "mcp")]
            self.process_mcp(ctx);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.render(ui);
    }
}

impl Editor {
    #[cfg(feature = "mcp")]
    fn process_mcp(&mut self, ctx: &egui::Context) {
        if let Some(mut bridge) = self.mcp.take() {
            if !self.native_input_pending && bridge.take_close_request() {
                self.request_close();
                ctx.request_repaint();
            }
            bridge.dispatch(self, ctx);
            self.mcp = Some(bridge);
        }
    }
}

fn unsupported(ui: &mut egui::Ui, label: &str) {
    ui.add_enabled(false, egui::Button::new(label))
        .on_disabled_hover_text("Not implemented in this build");
}

fn text_edit_focused(ctx: &egui::Context) -> bool {
    ctx.memory(|memory| {
        memory.focused().is_some_and(|id| {
            id == name_edit_id()
                || id == material_edit_id()
                || (0..9).any(|index| id == channel_edit_id(index))
        })
    })
}

fn name_edit_id() -> egui::Id {
    egui::Id::new("node_name")
}

fn channel_edit_id(index: usize) -> egui::Id {
    egui::Id::new(("channel", index))
}

fn material_edit_id() -> egui::Id {
    egui::Id::new("material_diffuse")
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "../snapshots.rs"]
pub(crate) mod snapshots;

#[cfg(test)]
#[path = "../entry_point_tests.rs"]
mod entry_point_tests;

#[cfg(test)]
#[path = "../guard_matrix_tests.rs"]
mod guard_matrix_tests;

#[cfg(all(test, feature = "mcp"))]
#[path = "../mcp_tests/mod.rs"]
mod mcp_editor_tests;

#[cfg(test)]
#[path = "../gpu_viewport_tests.rs"]
mod gpu_viewport_tests;
