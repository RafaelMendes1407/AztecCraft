use super::Editor;
use crate::editor_operations::{
    ActiveTool, MoveDistance, Operation, OperationError, OperationReply,
};
use azteccraft::{
    history::{EditOutcome, MoveFinish, MoveId, MoveProvenance},
    move_handles::{AxisDrag, ViewportCapture, WorldHandles},
    native_scene::{TransformTarget, WorldAxis},
};
use glam::Vec2;
use uuid::Uuid;

pub(super) struct NativeMove {
    id: MoveId,
    scene_token: Uuid,
    drag: AxisDrag,
}

// Both fields are read only by the cfg-gated inspect_move below.
#[cfg_attr(not(any(test, feature = "mcp")), allow(dead_code))]
pub(super) struct MoveViewportFeedback {
    capture: ViewportCapture,
    pointer: Option<Vec2>,
}

fn axis_name(axis: WorldAxis) -> &'static str {
    match axis {
        WorldAxis::X => "x",
        WorldAxis::Y => "y",
        WorldAxis::Z => "z",
    }
}

fn edit_reply(outcome: EditOutcome) -> OperationReply {
    match outcome {
        EditOutcome::Changed | EditOutcome::Created(_) => OperationReply::Changed,
        EditOutcome::Unchanged => OperationReply::Unchanged,
    }
}

impl Editor {
    pub(super) fn selected_move_target(&self) -> Result<TransformTarget, OperationError> {
        let node = self.selected.ok_or(OperationError::NoSelection)?;
        Ok(TransformTarget {
            node,
            path: self.selected_path.clone(),
        })
    }

    // Returns whether a Move was actually cancelled. Only then is the current pointer sequence
    // consumed; an idle cancel must not swallow the next frame's input.
    pub(super) fn cancel_move(&mut self) -> Result<bool, OperationError> {
        let cancelled = self.history.active_move().is_some() || self.native_move.is_some();
        let result = if self.history.active_move().is_some() {
            self.history
                .cancel_active_move()
                .map(|_| ())
                .map_err(OperationError::Move)
        } else {
            Ok(())
        };
        self.native_move = None;
        if cancelled {
            self.move_consumed_frame = true;
            self.move_suppress_primary = true;
        }
        result?;
        if cancelled {
            self.sync_fields();
        }
        Ok(cancelled)
    }

    pub(super) fn consume_pointer_sequence(&mut self) {
        self.move_consumed_frame = true;
        self.move_suppress_primary = true;
    }

    pub(super) fn guard_move_dispatch(
        &mut self,
        operation: &Operation,
        provenance: MoveProvenance,
    ) -> Result<(), OperationError> {
        if provenance == MoveProvenance::Native
            && matches!(
                operation,
                Operation::SetActiveTool(_)
                    | Operation::Select { .. }
                    | Operation::NewScene
                    | Operation::OpenScene { .. }
                    | Operation::RequestClose
            )
        {
            let tool_change =
                matches!(operation, Operation::SetActiveTool(tool) if *tool != self.active_tool);
            let cancelled = self.cancel_move()?;
            if tool_change && !cancelled {
                self.consume_pointer_sequence();
            }
            return Ok(());
        }
        if self.history.active_move().is_none() {
            return Ok(());
        }
        if matches!(
            operation,
            Operation::UpdateMove { .. } | Operation::FinishMove { .. }
        ) {
            return Ok(());
        }
        Err(OperationError::BusyManipulation)
    }

    pub(super) fn set_active_tool(
        &mut self,
        tool: ActiveTool,
    ) -> Result<OperationReply, OperationError> {
        let changed = self.active_tool != tool;
        self.active_tool = tool;
        Ok(if changed {
            OperationReply::Changed
        } else {
            OperationReply::Unchanged
        })
    }

    pub(super) fn begin_editor_move(
        &mut self,
        target: &TransformTarget,
        axis: WorldAxis,
        provenance: MoveProvenance,
    ) -> Result<OperationReply, OperationError> {
        if self.selected != Some(target.node) || self.selected_path != target.path {
            return Err(OperationError::InvalidInput(
                "Move target must be the exact selected transform instance".into(),
            ));
        }
        let id = self
            .history
            .begin_move(
                target,
                axis,
                self.scene_token,
                provenance,
                256 * 1024 * 1024,
            )
            .map_err(OperationError::Move)?;
        self.sync_fields();
        Ok(OperationReply::MoveStarted {
            move_id: id.to_decimal(),
        })
    }

    // Without the Declared arm the distance match is a single-arm destructure.
    #[cfg_attr(
        not(any(test, feature = "mcp")),
        allow(clippy::infallible_destructuring_match)
    )]
    pub(super) fn update_editor_move(
        &mut self,
        id: MoveId,
        distance: MoveDistance,
        provenance: MoveProvenance,
    ) -> Result<OperationReply, OperationError> {
        let factor = self
            .history
            .scene()
            .units()
            .linear_factor()
            .map_err(OperationError::Scene)?;
        let distance_cm = match distance {
            MoveDistance::Centimeters(value) => value,
            #[cfg(any(test, feature = "mcp"))]
            MoveDistance::Declared(value) => {
                if !value.is_finite() {
                    return Err(OperationError::InvalidInput(
                        "Move distance must be finite".into(),
                    ));
                }
                value * factor
            }
        };
        if !distance_cm.is_finite() || !(distance_cm / factor).is_finite() {
            return Err(OperationError::InvalidInput(
                "Move distance cannot be represented in the declared linear unit".into(),
            ));
        }
        let outcome = self
            .history
            .update_move(self.scene_token, id, provenance, distance_cm)
            .map_err(OperationError::Move)?;
        self.sync_fields();
        Ok(edit_reply(outcome))
    }

    pub(super) fn finish_editor_move(
        &mut self,
        id: MoveId,
        finish: MoveFinish,
        provenance: MoveProvenance,
    ) -> Result<OperationReply, OperationError> {
        let outcome = self
            .history
            .finish_move(self.scene_token, id, provenance, finish)
            .map_err(OperationError::Move)?;
        self.native_move = None;
        self.sync_fields();
        Ok(edit_reply(outcome))
    }

    #[cfg(any(test, feature = "mcp"))]
    pub(super) fn inspect_move(
        &self,
    ) -> Result<crate::editor_operations::MoveInspection, OperationError> {
        use crate::editor_operations::{
            ActiveMoveInspection, MoveAxisInspection, MoveInspection, MoveViewportInspection,
        };
        let frame = self.selected_move_target().and_then(|target| {
            self.history
                .scene()
                .world_move_frame(&target)
                .map_err(OperationError::Scene)
        });
        let numeric_unavailable_reason = frame.as_ref().err().map(ToString::to_string);
        let active = self
            .history
            .active_move()
            .map(|active| {
                let factor = self
                    .history
                    .scene()
                    .units()
                    .linear_factor()
                    .map_err(OperationError::Scene)?;
                let distance = active.distance_cm / factor;
                if !distance.is_finite() {
                    return Err(OperationError::InvalidInput(
                        "Move distance inspection overflow".into(),
                    ));
                }
                Ok(ActiveMoveInspection {
                    move_id: active.id.to_decimal(),
                    node_id: active.target.node.to_decimal(),
                    instance_path: active
                        .target
                        .path
                        .iter()
                        .map(|id| id.to_decimal())
                        .collect(),
                    axis: axis_name(active.axis),
                    provenance: match active.provenance {
                        MoveProvenance::Native => "native",
                        MoveProvenance::Remote => "remote",
                    },
                    distance,
                })
            })
            .transpose()?;
        let viewport = self
            .move_viewport
            .as_ref()
            .filter(|feedback| {
                feedback.capture.same_camera(&self.camera)
                    && !self
                        .renderer
                        .as_ref()
                        .is_some_and(|renderer| renderer.terminal_error().is_some())
            })
            .and_then(|feedback| frame.as_ref().ok().map(|frame| (feedback, frame)))
            .map(|(feedback, frame)| {
                let geometry = WorldHandles::new(&feedback.capture, frame.origin_cm());
                let pointer = feedback
                    .pointer
                    .and_then(|pointer| feedback.capture.pointer_pixel(pointer).ok());
                let pointer_hit = geometry
                    .as_ref()
                    .ok()
                    .and_then(|handles| pointer.and_then(|pixel| handles.hit_test(pixel)))
                    .map(axis_name);
                let axes = match &geometry {
                    Ok(handles) => handles
                        .axes()
                        .iter()
                        .map(|handle| MoveAxisInspection {
                            axis: axis_name(handle.axis),
                            visible: handle.arrow().is_some(),
                            unavailable_reason: handle
                                .unavailable_reason()
                                .map(|error| error.to_string()),
                        })
                        .collect(),
                    Err(error) => [WorldAxis::X, WorldAxis::Y, WorldAxis::Z]
                        .into_iter()
                        .map(|axis| MoveAxisInspection {
                            axis: axis_name(axis),
                            visible: false,
                            unavailable_reason: Some(error.to_string()),
                        })
                        .collect(),
                };
                MoveViewportInspection {
                    provenance: "last_native_viewport_capture",
                    axes,
                    pointer_hit,
                    pointer_available: pointer.is_some(),
                }
            });
        Ok(MoveInspection {
            active_tool: self.active_tool,
            orientation: "world",
            class: "object_translation",
            numeric_available: frame.is_ok(),
            numeric_unavailable_reason,
            active,
            viewport,
        })
    }
}

impl Editor {
    pub(super) fn missing_move_viewport(&mut self) -> Result<(), OperationError> {
        self.move_viewport = None;
        if self.native_move.is_some() {
            self.cancel_move()?;
            self.status = "Move cancelled: viewport capture is unavailable".into();
            self.error = true;
        }
        Ok(())
    }

    // A refused final sample leaves the previous preview intact, so release commits that preview.
    fn finish_native_sample(
        &mut self,
        id: MoveId,
        released: bool,
        updated: Result<OperationReply, OperationError>,
    ) -> Result<(), OperationError> {
        if !released {
            updated?;
            self.status = "World-axis Move preview".into();
            self.error = false;
            return Ok(());
        }
        self.apply_operation(Operation::FinishMove {
            id,
            finish: MoveFinish::Commit,
        })?;
        match updated {
            Ok(_) => {
                self.status = "Move committed".into();
                self.error = false;
            }
            Err(error) => {
                self.status = format!("{error}; last valid preview committed");
                self.error = true;
            }
        }
        Ok(())
    }

    pub(super) fn handle_move_input(
        &mut self,
        input: &crate::viewport_ui::ViewportInput,
        frame_token: Uuid,
    ) -> Result<bool, OperationError> {
        let pointer = input.move_pointer();
        let capture = match input.move_capture(&self.camera) {
            Ok(capture) => capture,
            Err(error) => {
                self.missing_move_viewport()?;
                return Err(OperationError::InvalidInput(error.to_string()));
            }
        };
        self.move_viewport = Some(MoveViewportFeedback {
            capture: capture.clone(),
            pointer: pointer.position.filter(|point| point.is_finite()),
        });
        if self.move_suppress_primary || self.move_consumed_frame {
            if !pointer.down {
                self.move_suppress_primary = false;
            }
            return Ok(true);
        }
        if let Some(native) = &self.native_move {
            let matching = native.scene_token == self.scene_token
                && frame_token == self.scene_token
                && self.history.active_move().is_some_and(|active| {
                    active.id == native.id && active.provenance == MoveProvenance::Native
                });
            if !matching {
                self.native_move = None;
                self.move_consumed_frame = true;
                self.move_suppress_primary = pointer.down;
                return Err(OperationError::Move(
                    azteccraft::history::MoveError::StaleMove,
                ));
            }
            if !native.drag.same_capture(&capture)
                || pointer.modified
                || !pointer.focused
                || pointer.gone
                || (!pointer.down && !pointer.released)
                || self.files.is_modal()
                || eframe::egui::Popup::is_any_open(input.painter().ctx())
                || self.files.is_closing()
                || self
                    .renderer
                    .as_ref()
                    .is_some_and(|renderer| renderer.terminal_error().is_some())
            {
                self.cancel_move()?;
                self.status = "Move cancelled: camera, viewport or native input changed".into();
                self.error = false;
                return Ok(true);
            }
            let Some(position) = pointer.position.filter(|point| point.is_finite()) else {
                self.cancel_move()?;
                self.status = "Move cancelled: pointer position is unavailable".into();
                self.error = true;
                return Ok(true);
            };
            let id = native.id;
            let sample = native.drag.distance_cm(&capture, position);
            match sample {
                Ok(distance) => {
                    let updated = self.apply_operation(Operation::UpdateMove {
                        id,
                        distance: MoveDistance::Centimeters(distance),
                    });
                    self.finish_native_sample(id, pointer.released, updated)?;
                }
                Err(
                    error @ (azteccraft::move_handles::HandleError::ParallelAxis
                    | azteccraft::move_handles::HandleError::BehindRay
                    | azteccraft::move_handles::HandleError::InvalidSolve),
                ) => {
                    if pointer.released {
                        self.apply_operation(Operation::FinishMove {
                            id,
                            finish: MoveFinish::Commit,
                        })?;
                        self.status = format!("{error}; last valid position committed");
                    } else {
                        self.status = error.to_string();
                    }
                    self.error = true;
                }
                Err(error) => {
                    self.cancel_move()?;
                    self.status = error.to_string();
                    self.error = true;
                }
            }
            return Ok(true);
        }
        if self.history.active_move().is_some() {
            if pointer.pressed {
                self.cancel_move()?;
                self.status = "Move cancelled by native viewport input".into();
                self.error = false;
            }
            return Ok(true);
        }
        if self.active_tool != ActiveTool::Move
            || !pointer.pressed
            || pointer.modified
            || !pointer.focused
            || self.files.is_modal()
            || eframe::egui::Popup::is_any_open(input.painter().ctx())
            || self.files.is_closing()
            || frame_token != self.scene_token
            || self
                .renderer
                .as_ref()
                .is_some_and(|renderer| renderer.terminal_error().is_some())
        {
            return Ok(false);
        }
        if !self.commit_pending() {
            self.move_consumed_frame = true;
            self.move_suppress_primary = true;
            return Err(OperationError::InvalidInput(self.status.clone()));
        }
        let Some(position) = pointer.position.filter(|point| point.is_finite()) else {
            return Ok(false);
        };
        let Ok(target) = self.selected_move_target() else {
            return Ok(false);
        };
        let Ok(frame) = self.history.scene().world_move_frame(&target) else {
            return Ok(false);
        };
        let Ok(handles) = WorldHandles::new(&capture, frame.origin_cm()) else {
            return Ok(false);
        };
        let pixel = capture
            .pointer_pixel(position)
            .map_err(|error| OperationError::InvalidInput(error.to_string()))?;
        let Some(axis) = handles.hit_test(pixel) else {
            return Ok(false);
        };
        let drag = AxisDrag::begin(capture, frame.origin_cm(), axis, position)
            .map_err(|error| OperationError::InvalidInput(error.to_string()))?;
        self.apply_operation(Operation::BeginMove { target, axis })?;
        let id = self
            .history
            .active_move()
            .ok_or(OperationError::Move(
                azteccraft::history::MoveError::StaleMove,
            ))?
            .id;
        self.native_move = Some(NativeMove {
            id,
            scene_token: self.scene_token,
            drag,
        });
        if pointer.released {
            self.apply_operation(Operation::FinishMove {
                id,
                finish: MoveFinish::Commit,
            })?;
        }
        self.status = format!("World {} Move", axis_name(axis).to_uppercase());
        self.error = false;
        Ok(true)
    }

    pub(super) fn paint_move(&self, input: &crate::viewport_ui::ViewportInput) {
        if self.active_tool != ActiveTool::Move
            || self
                .renderer
                .as_ref()
                .is_some_and(|renderer| renderer.terminal_error().is_some())
        {
            return;
        }
        let geometry = (|| {
            let target = self.selected_move_target()?;
            let frame = self
                .history
                .scene()
                .world_move_frame(&target)
                .map_err(OperationError::Scene)?;
            let capture = input
                .move_capture(&self.camera)
                .map_err(|error| OperationError::InvalidInput(error.to_string()))?;
            let handles = WorldHandles::new(&capture, frame.origin_cm())
                .map_err(|error| OperationError::InvalidInput(error.to_string()))?;
            Ok::<_, OperationError>((capture, handles))
        })();
        let painter = input.painter().with_clip_rect(input.rect());
        let scale = input.pixels_per_point();
        let (capture, handles) = match geometry {
            Ok(geometry) => geometry,
            Err(error) => {
                painter.text(
                    input.rect().left_top() + eframe::egui::vec2(12.0, 12.0),
                    eframe::egui::Align2::LEFT_TOP,
                    format!("Move unavailable: {error}"),
                    eframe::egui::FontId::proportional(12.0),
                    eframe::egui::Color32::LIGHT_GRAY,
                );
                return;
            }
        };
        let active = self.history.active_move().map(|active| active.axis);
        let hovered = if active.is_none() {
            input
                .move_pointer()
                .position
                .and_then(|pointer| capture.pointer_pixel(pointer).ok())
                .and_then(|pixel| handles.hit_test(pixel))
        } else {
            None
        };
        for handle in handles.axes() {
            let Some(arrow) = handle.arrow() else {
                continue;
            };
            let color = if active == Some(handle.axis) {
                eframe::egui::Color32::from_rgb(255, 225, 80)
            } else if hovered == Some(handle.axis) {
                eframe::egui::Color32::WHITE
            } else {
                match handle.axis {
                    WorldAxis::X => eframe::egui::Color32::from_rgb(235, 75, 75),
                    WorldAxis::Y => eframe::egui::Color32::from_rgb(90, 220, 105),
                    WorldAxis::Z => eframe::egui::Color32::from_rgb(85, 150, 245),
                }
            };
            let point = |pixel: glam::DVec2| {
                input.rect().min
                    + eframe::egui::vec2(pixel.x as f32 / scale, pixel.y as f32 / scale)
            };
            for segment in arrow.segments() {
                painter.line_segment(
                    [point(segment.start), point(segment.end)],
                    eframe::egui::Stroke::new(2.0 / scale, color),
                );
            }
            painter.text(
                point(arrow.label()),
                eframe::egui::Align2::CENTER_CENTER,
                axis_name(handle.axis).to_uppercase(),
                eframe::egui::FontId::proportional(11.0 / scale),
                color,
            );
        }
    }
}

#[cfg(test)]
#[path = "move_ui/tests.rs"]
mod tests;
