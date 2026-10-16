use super::{Command, EditOutcome, History};
use crate::native_scene::{
    MoveTranslationBaseline, NativeError, NativeId, TransformTarget, WorldAxis, WorldMoveFrame,
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
    StaleMove,
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
            Self::StaleMove => f.write_str("Move identifier is absent or stale"),
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

pub(super) struct MoveTransaction {
    id: ManipulationId,
    scene_token: Uuid,
    target: TransformTarget,
    axis: WorldAxis,
    provenance: ManipulationProvenance,
    frame: WorldMoveFrame,
    baseline: MoveTranslationBaseline,
    view_budget: usize,
    distance_cm: f64,
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
            return Err(ManipulationError::StaleMove);
        }
        if provenance != self.provenance {
            return Err(ManipulationError::WrongOwner);
        }
        Ok(())
    }
}

impl History {
    pub fn active_manipulation(&self) -> Option<MoveState<'_>> {
        self.active_manipulation.as_ref().map(|active| MoveState {
            id: active.id,
            target: &active.target,
            axis: active.axis,
            provenance: active.provenance,
            distance_cm: active.distance_cm,
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
            axis,
            provenance,
            frame,
            baseline,
            view_budget,
            distance_cm: 0.0,
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
            .ok_or(ManipulationError::StaleMove)?;
        active.matches(scene_token, id, provenance)?;
        let translation = active.frame.translation(active.axis, distance_cm)?;
        let (previous, changed) = self
            .scene
            .preview_move_translation(&active.baseline, translation)?;
        if let Err(error) = ViewScene::from_native(&self.scene, active.view_budget) {
            self.scene.restore_move_translation(&previous)?;
            return Err(error.into());
        }
        active.distance_cm = distance_cm;
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
            .ok_or(ManipulationError::StaleMove)?
            .matches(scene_token, id, provenance)?;
        // Clear before any fallible scene work so a failure cannot leave an unfinishable move.
        let active = self
            .active_manipulation
            .take()
            .ok_or(ManipulationError::StaleMove)?;
        let channels = self.scene.move_translation_changes(&active.baseline)?;
        let node = active.target.node;
        self.scene.restore_move_translation(&active.baseline)?;
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
        let channels = self.scene.move_translation_changes(&active.baseline)?;
        self.scene.restore_move_translation(&active.baseline)?;
        Ok(if channels.iter().all(Option::is_none) {
            EditOutcome::Unchanged
        } else {
            EditOutcome::Changed
        })
    }
}

#[cfg(test)]
mod tests;
