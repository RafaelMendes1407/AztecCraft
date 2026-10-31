//! Current time, playback range and playback: session state of the editor, reset on New and Open
//! and never dirtying by itself. The keys themselves live in `NativeScene`. Playback runs as a tick
//! in `render`, not as an `Operation`, so it cannot fill the undo queue.

use crate::app::*;
use azteccraft::native_scene::{ChannelMask, NativeError, whole_frame};

/// PROVISIONAL: the film time unit is 24 fps, and real-time playback is an assumed default.
pub(in crate::app) const FRAMES_PER_SECOND: f64 = 24.0;
/// The Maya 2027 New Scene Options default animation start and end. The docs state no playback
/// range default, so this range starts equal to them (PROVISIONAL).
const DEFAULT_START: i32 = 1;
const DEFAULT_END: i32 = 48;

pub(in crate::app) struct Timeline {
    pub(in crate::app) current: i32,
    pub(in crate::app) start: i32,
    pub(in crate::app) end: i32,
    pub(in crate::app) playing: bool,
    /// The wall-clock second and frame playback counts from; the first tick sets it.
    anchor: Option<(f64, i32)>,
}

impl Default for Timeline {
    fn default() -> Self {
        Self {
            current: DEFAULT_START,
            start: DEFAULT_START,
            end: DEFAULT_END,
            playing: false,
            anchor: None,
        }
    }
}

impl Timeline {
    /// The frame `now` seconds into playback, looping continuously over the range (PROVISIONAL:
    /// the docs state no default loop mode). `None` when no frame can be derived from `now`.
    fn frame_at(&self, now: f64, anchor: (f64, i32)) -> Option<i32> {
        let elapsed = now - anchor.0;
        if !elapsed.is_finite() || elapsed < 0.0 {
            return None;
        }
        let length = f64::from(self.end - self.start) + 1.0;
        let played = (elapsed * FRAMES_PER_SECOND).floor() + f64::from(anchor.1 - self.start);
        Some(self.start + played.rem_euclid(length) as i32)
    }
}

/// Whether an operation stops playback before it runs. Reads, selection, the camera and view
/// settings leave the scene alone and keep playing; a new operation stops it by default.
pub(in crate::app) fn stops_playback(operation: &Operation) -> bool {
    match operation {
        Operation::Play
        | Operation::Stop
        | Operation::SetActiveTool(_)
        | Operation::SetToolSpace { .. }
        | Operation::SetToolHandle(_)
        | Operation::SetManipulatorSize(_)
        | Operation::Select { .. }
        | Operation::ModifySelection { .. }
        | Operation::SelectByRule(_)
        | Operation::SetSelectionMode(_)
        | Operation::DeselectAll
        | Operation::SelectFace { .. }
        | Operation::SelectVertex { .. }
        | Operation::SelectEdge { .. }
        | Operation::FrameSelected { .. }
        | Operation::FrameAll
        | Operation::SetGridVisible { .. }
        | Operation::SetShadingMode(_)
        | Operation::SetSidebarPanelVisible { .. }
        | Operation::SelectAttributeEditorTab { .. }
        | Operation::Orbit { .. }
        | Operation::Pan { .. }
        | Operation::Dolly { .. }
        | Operation::SetCameraPose { .. } => false,
        #[cfg(any(test, feature = "mcp"))]
        Operation::Inspect { .. } | Operation::InspectKeys { .. } => false,
        _ => true,
    }
}

impl Editor {
    /// Writes `frame`'s pose, then moves the time and refreshes the fields. Callers check for a
    /// draft first: a stale field would read as a draft, and a draft would be lost to the refresh.
    fn write_time(&mut self, frame: i32) -> Result<(), OperationError> {
        let wrote = self
            .history
            .apply_evaluated(frame)
            .map_err(OperationError::from_history)?;
        self.timeline.current = frame;
        if wrote {
            self.sync_fields();
        }
        Ok(())
    }

    pub(in crate::app) fn set_current_time(
        &mut self,
        time: f64,
    ) -> Result<OperationReply, OperationError> {
        self.history
            .scene()
            .check_time_unit()
            .map_err(OperationError::Scene)?;
        let frame = whole_frame(time).map_err(OperationError::Scene)?;
        if self.has_draft() {
            return Err(OperationError::BusyEditing);
        }
        if frame == self.timeline.current {
            return Ok(OperationReply::Unchanged);
        }
        self.write_time(frame)?;
        Ok(OperationReply::Changed)
    }

    pub(in crate::app) fn set_playback_range(
        &mut self,
        start: f64,
        end: f64,
    ) -> Result<OperationReply, OperationError> {
        self.history
            .scene()
            .check_time_unit()
            .map_err(OperationError::Scene)?;
        let start = whole_frame(start).map_err(OperationError::Scene)?;
        let end = whole_frame(end).map_err(OperationError::Scene)?;
        if start > end {
            return Err(OperationError::Scene(NativeError::InvalidPlaybackRange));
        }
        let changed = (start, end) != (self.timeline.start, self.timeline.end);
        self.timeline.start = start;
        self.timeline.end = end;
        Ok(if changed {
            OperationReply::Changed
        } else {
            OperationReply::Unchanged
        })
    }

    /// Starts playback from the current time, or from the range start when that is outside the
    /// range (PROVISIONAL). A draft refuses it: playback never commits one.
    pub(in crate::app) fn play(&mut self) -> Result<OperationReply, OperationError> {
        self.history
            .scene()
            .check_time_unit()
            .map_err(OperationError::Scene)?;
        if self.has_draft() {
            return Err(OperationError::BusyEditing);
        }
        if self.timeline.playing {
            return Ok(OperationReply::Unchanged);
        }
        if !(self.timeline.start..=self.timeline.end).contains(&self.timeline.current) {
            self.write_time(self.timeline.start)?;
        }
        self.timeline.playing = true;
        self.timeline.anchor = None;
        Ok(OperationReply::Changed)
    }

    pub(in crate::app) fn stop(&mut self) -> OperationReply {
        let was_playing = self.timeline.playing;
        self.stop_playback();
        if was_playing {
            OperationReply::Changed
        } else {
            OperationReply::Unchanged
        }
    }

    pub(in crate::app) fn stop_playback(&mut self) {
        self.timeline.playing = false;
        self.timeline.anchor = None;
    }

    /// Why playback cannot go on: each of these owns the fields or the pose a tick would rewrite.
    fn playback_blocker(&self, text_focused: bool) -> Option<&'static str> {
        if self.files.is_closing() {
            Some("the window is closing")
        } else if self.files.is_modal() {
            Some("a file action needs an answer")
        } else if self.history.active_manipulation().is_some() || self.native_move.is_some() {
            Some("a manipulation is active")
        } else if text_focused {
            Some("a text field is focused")
        } else if self.has_draft() {
            Some("a field holds a draft")
        } else {
            None
        }
    }

    /// One playback step at `now` seconds. A frame is skipped rather than slowing playback down,
    /// so several may pass between two ticks, and a blocker stops playback before any write.
    pub(in crate::app) fn tick_playback(&mut self, now: f64, text_focused: bool) {
        if !self.timeline.playing {
            return;
        }
        if let Some(reason) = self.playback_blocker(text_focused) {
            self.stop_playback();
            self.status = format!("Playback stopped: {reason}");
            self.error = false;
            return;
        }
        let anchor = self.timeline.anchor.unwrap_or((now, self.timeline.current));
        let Some(frame) = self.timeline.frame_at(now, anchor) else {
            self.timeline.anchor = Some((now, self.timeline.current));
            return;
        };
        self.timeline.anchor = Some(anchor);
        if frame == self.timeline.current {
            return;
        }
        if let Err(error) = self.write_time(frame) {
            self.stop_playback();
            self.set_error_status(&error);
            self.error = true;
        }
    }

    /// Esc stops playback before any of its other meanings.
    pub(in crate::app) fn escape_stops_playback(&mut self, ctx: &egui::Context) -> bool {
        let stops = self.timeline.playing && ctx.input(|input| input.key_pressed(Key::Escape));
        if stops {
            self.stop_playback();
            self.status = "Playback stopped".into();
            self.error = false;
        }
        stops
    }

    /// Refuses a key request before a draft commit. Omitted nodes mean the object selection, which
    /// a component mode does not offer.
    pub(in crate::app) fn check_key_request(
        &self,
        nodes: Option<&[NodeId]>,
        time: Option<f64>,
        channels: ChannelMask,
    ) -> Result<(), OperationError> {
        self.history
            .scene()
            .check_time_unit()
            .map_err(OperationError::Scene)?;
        if let Some(time) = time {
            whole_frame(time).map_err(OperationError::Scene)?;
        }
        if !channels.contains(&true) {
            return Err(OperationError::InvalidInput(
                "Name at least one channel".into(),
            ));
        }
        match nodes {
            Some([]) => Err(OperationError::InvalidInput(
                "Name at least one object".into(),
            )),
            Some(_) => Ok(()),
            None if self.selection_mode.is_component() => Err(OperationError::InvalidInput(
                "Keys need Object or Hierarchy selection mode".into(),
            )),
            None if self.selection.is_empty() => Err(OperationError::InvalidInput(
                "Select an object to key".into(),
            )),
            None => Ok(()),
        }
    }

    fn key_nodes(&self, nodes: Option<Vec<NodeId>>) -> Vec<NodeId> {
        nodes.unwrap_or_else(|| self.selection.members().iter().map(|t| t.node).collect())
    }

    pub(in crate::app) fn set_key_command(
        &self,
        nodes: Option<Vec<NodeId>>,
        time: Option<f64>,
        channels: ChannelMask,
    ) -> Result<Command, OperationError> {
        let current_frame = self.timeline.current;
        Ok(Command::SetKeys {
            nodes: self.key_nodes(nodes),
            frame: time
                .map(whole_frame)
                .transpose()
                .map_err(OperationError::Scene)?
                .unwrap_or(current_frame),
            channels,
            current_frame,
        })
    }

    pub(in crate::app) fn delete_keys_command(
        &self,
        nodes: Option<Vec<NodeId>>,
        time: Option<f64>,
        channels: ChannelMask,
    ) -> Result<Command, OperationError> {
        Ok(Command::DeleteKeys {
            nodes: self.key_nodes(nodes),
            frame: time
                .map(whole_frame)
                .transpose()
                .map_err(OperationError::Scene)?,
            channels,
            current_frame: self.timeline.current,
        })
    }

    /// One page of keys. A read passes no token guard, so the cursor binds the document and the
    /// revision; a playback tick that moves a keyed channel makes it stale.
    #[cfg(any(test, feature = "mcp"))]
    pub(in crate::app) fn inspect_keys(
        &self,
        node: Option<NodeId>,
        cursor: Option<&crate::editor_operations::PageCursor>,
    ) -> Result<crate::editor_operations::KeysInspection, OperationError> {
        use crate::editor_operations::{
            KeyInspection, KeysInspection, MAX_KEYS_PER_PAGE, PageCursor,
        };
        use azteccraft::native_scene::KEY_CHANNELS;
        let scene = self.history.scene();
        if let Some(node) = node {
            scene.node(node).map_err(OperationError::Scene)?;
        }
        let keys_total = scene.key_rows(node).count();
        let key_offset = match cursor {
            None => 0,
            Some(cursor) => {
                if cursor.scene_token != self.scene_token
                    || cursor.revision != self.history.revision()
                {
                    return Err(OperationError::StaleScene);
                }
                if cursor.offset > keys_total {
                    return Err(OperationError::InvalidInput(
                        "cursor offset is past the last key".into(),
                    ));
                }
                cursor.offset
            }
        };
        let keys: Vec<KeyInspection> = scene
            .key_rows(node)
            .skip(key_offset)
            .take(MAX_KEYS_PER_PAGE)
            .map(|row| KeyInspection {
                node_id: row.node.to_decimal(),
                channel: KEY_CHANNELS[row.channel],
                frame: row.frame,
                value: row.value,
            })
            .collect();
        let end = key_offset + keys.len();
        Ok(KeysInspection {
            scene_token: self.scene_token.to_string(),
            revision: self.history.revision(),
            unavailable_reason: scene.has_source().then_some(
                "Animation curves of a sourced scene are not exposed or evaluated, which does not mean there are none",
            ),
            keys_total,
            key_offset,
            keys,
            next_cursor: (end < keys_total).then(|| {
                PageCursor {
                    scene_token: self.scene_token,
                    revision: self.history.revision(),
                    offset: end,
                }
                .to_string()
            }),
        })
    }

    #[cfg(any(test, feature = "mcp"))]
    pub(in crate::app) fn inspect_timeline(&self) -> crate::editor_operations::TimelineInspection {
        crate::editor_operations::TimelineInspection {
            current_time: self.timeline.current,
            playback_start: self.timeline.start,
            playback_end: self.timeline.end,
            playing: self.timeline.playing,
            frames_per_second: FRAMES_PER_SECOND,
            loop_mode: "continuous",
            key_count: self.history.scene().session_key_count(),
        }
    }
}

#[cfg(test)]
mod tests;
