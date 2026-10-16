use super::{Command, EditOutcome, History};
use crate::native_scene::{
    MoveTranslationBaseline, NativeError, NativeId, NativeScene, TransformTarget, WorldAxis,
    WorldMoveFrame,
};
use crate::view_scene::{ViewError, ViewScene};
use std::{fmt, mem::size_of};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManipulationId(u64);

impl ManipulationId {
    pub fn to_decimal(self) -> String {
        self.0.to_string()
    }

    pub fn from_decimal(value: &str) -> Result<Self, std::num::ParseIntError> {
        value.parse().map(Self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManipulationProvenance {
    Native,
    Remote,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManipulationFinish {
    Commit,
    Cancel,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ManipulationError {
    Native(NativeError),
    View(ViewError),
    Busy,
    StaleScene,
    StaleManipulation,
    WrongOwner,
    IdsExhausted,
}

impl From<NativeError> for ManipulationError {
    fn from(error: NativeError) -> Self {
        Self::Native(error)
    }
}

impl From<ViewError> for ManipulationError {
    fn from(error: ViewError) -> Self {
        Self::View(error)
    }
}

impl fmt::Display for ManipulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Native(error) => error.fmt(f),
            Self::View(error) => error.fmt(f),
            Self::Busy => f.write_str("finish or cancel the active Move manipulation"),
            Self::StaleScene => f.write_str("Move scene token is stale"),
            Self::StaleManipulation => f.write_str("Move identifier is absent or stale"),
            Self::WrongOwner => f.write_str("Move manipulation belongs to another input source"),
            Self::IdsExhausted => f.write_str("Move identifiers are exhausted"),
        }
    }
}

impl std::error::Error for ManipulationError {}

pub struct MoveState<'a> {
    pub id: ManipulationId,
    pub target: &'a TransformTarget,
    pub axis: WorldAxis,
    pub provenance: ManipulationProvenance,
    pub distance_cm: f64,
}

/// The kind of manipulation and its kind-specific state; the guards in `History` are shared.
enum ManipulationKind {
    Move {
        axis: WorldAxis,
        frame: WorldMoveFrame,
        baseline: MoveTranslationBaseline,
    },
}

/// Exact scene state captured by a preview so a failed one can be undone.
enum ManipulationSnapshot {
    Move(MoveTranslationBaseline),
}

impl ManipulationKind {
    fn preview(
        &self,
        scene: &mut NativeScene,
        parameter: f64,
    ) -> Result<(ManipulationSnapshot, bool), NativeError> {
        match self {
            Self::Move {
                axis,
                frame,
                baseline,
            } => {
                let translation = frame.translation(*axis, parameter)?;
                let (previous, changed) = scene.preview_move_translation(baseline, translation)?;
                Ok((ManipulationSnapshot::Move(previous), changed))
            }
        }
    }

    fn restore(&self, scene: &mut NativeScene) -> Result<(), NativeError> {
        match self {
            Self::Move { baseline, .. } => scene.restore_move_translation(baseline),
        }
    }

    fn changes(&self, scene: &NativeScene) -> Result<[Option<f64>; 9], NativeError> {
        match self {
            Self::Move { baseline, .. } => scene.move_translation_changes(baseline),
        }
    }
}

impl ManipulationSnapshot {
    fn restore(&self, scene: &mut NativeScene) -> Result<(), NativeError> {
        match self {
            Self::Move(previous) => scene.restore_move_translation(previous),
        }
    }
}

pub(super) struct MoveTransaction {
    id: ManipulationId,
    scene_token: Uuid,
    target: TransformTarget,
    provenance: ManipulationProvenance,
    kind: ManipulationKind,
    view_budget: usize,
    parameter: f64,
}

impl MoveTransaction {
    fn matches(
        &self,
        scene_token: Uuid,
        id: ManipulationId,
        provenance: ManipulationProvenance,
    ) -> Result<(), ManipulationError> {
        if scene_token != self.scene_token {
            return Err(ManipulationError::StaleScene);
        }
        if id != self.id {
            return Err(ManipulationError::StaleManipulation);
        }
        if provenance != self.provenance {
            return Err(ManipulationError::WrongOwner);
        }
        Ok(())
    }
}

impl History {
    pub fn active_manipulation(&self) -> Option<MoveState<'_>> {
        self.active_manipulation.as_ref().map(|active| {
            let ManipulationKind::Move { axis, .. } = &active.kind;
            MoveState {
                id: active.id,
                target: &active.target,
                axis: *axis,
                provenance: active.provenance,
                distance_cm: active.parameter,
            }
        })
    }

    pub fn begin_move(
        &mut self,
        target: &TransformTarget,
        axis: WorldAxis,
        scene_token: Uuid,
        provenance: ManipulationProvenance,
        byte_budget: usize,
    ) -> Result<ManipulationId, ManipulationError> {
        if self.active_manipulation.is_some() {
            return Err(ManipulationError::Busy);
        }
        let next_id = self
            .next_manipulation_id
            .checked_add(1)
            .ok_or(ManipulationError::IdsExhausted)?;
        let (frame, baseline) = self.scene.begin_move_translation(target, axis)?;
        let mut path = Vec::new();
        path.try_reserve_exact(target.path.len())
            .map_err(|_| NativeError::ResourceLimit)?;
        path.extend_from_slice(&target.path);
        let metadata = path
            .capacity()
            .checked_mul(size_of::<NativeId>())
            .and_then(|bytes| bytes.checked_add(size_of::<MoveTransaction>()))
            .ok_or(ViewError::ResourceLimit)?;
        let view_budget = byte_budget
            .checked_sub(metadata)
            .ok_or(ViewError::ResourceLimit)?;
        ViewScene::from_native(&self.scene, view_budget)?;
        self.active_manipulation = Some(MoveTransaction {
            id: ManipulationId(next_id),
            scene_token,
            target: TransformTarget {
                node: target.node,
                path,
            },
            provenance,
            kind: ManipulationKind::Move {
                axis,
                frame,
                baseline,
            },
            view_budget,
            parameter: 0.0,
        });
        self.next_manipulation_id = next_id;
        Ok(ManipulationId(next_id))
    }

    pub fn update_move(
        &mut self,
        scene_token: Uuid,
        id: ManipulationId,
        provenance: ManipulationProvenance,
        distance_cm: f64,
    ) -> Result<EditOutcome, ManipulationError> {
        let active = self
            .active_manipulation
            .as_mut()
            .ok_or(ManipulationError::StaleManipulation)?;
        active.matches(scene_token, id, provenance)?;
        let (previous, changed) = active.kind.preview(&mut self.scene, distance_cm)?;
        if let Err(error) = ViewScene::from_native(&self.scene, active.view_budget) {
            previous.restore(&mut self.scene)?;
            return Err(error.into());
        }
        active.parameter = distance_cm;
        Ok(if changed {
            EditOutcome::Changed
        } else {
            EditOutcome::Unchanged
        })
    }

    pub fn finish_move(
        &mut self,
        scene_token: Uuid,
        id: ManipulationId,
        provenance: ManipulationProvenance,
        finish: ManipulationFinish,
    ) -> Result<EditOutcome, ManipulationError> {
        self.active_manipulation
            .as_ref()
            .ok_or(ManipulationError::StaleManipulation)?
            .matches(scene_token, id, provenance)?;
        // Clear before any fallible scene work so a failure cannot leave an unfinishable move.
        let active = self
            .active_manipulation
            .take()
            .ok_or(ManipulationError::StaleManipulation)?;
        let channels = active.kind.changes(&self.scene)?;
        let node = active.target.node;
        active.kind.restore(&mut self.scene)?;
        if finish == ManipulationFinish::Cancel || channels.iter().all(Option::is_none) {
            return Ok(if channels.iter().all(Option::is_none) {
                EditOutcome::Unchanged
            } else {
                EditOutcome::Changed
            });
        }
        self.execute(Command::EditNode {
            id: node,
            name: None,
            channels,
        })
        .map_err(Into::into)
    }

    pub fn cancel_active_move(&mut self) -> Result<EditOutcome, ManipulationError> {
        // Clear before any fallible scene work so a failure cannot leave an unfinishable move.
        let Some(active) = self.active_manipulation.take() else {
            return Ok(EditOutcome::Unchanged);
        };
        let channels = active.kind.changes(&self.scene)?;
        active.kind.restore(&mut self.scene)?;
        Ok(if channels.iter().all(Option::is_none) {
            EditOutcome::Unchanged
        } else {
            EditOutcome::Changed
        })
    }
}

#[cfg(test)]
mod tests;
