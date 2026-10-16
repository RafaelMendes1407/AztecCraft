use super::move_translation::{ChannelBaseline, MIN_RESOLUTION, ROUNDTRIP_EPSILON, ulp_bound};
use super::*;
use glam::DMat3;
use source::Storage;
use std::f64::consts::{PI, TAU};

// Euler decomposition is refused this close to gimbal lock (|cos y| below the bound).
const MIN_COS_PITCH: f64 = 1e-6;
// Non-intended channels this close to their baseline are restored to the baseline bits.
const SNAP_TOLERANCE: f64 = 1e-12;
const MAX_ANGLE_RADIANS: f64 = 1e6;

/// Object-space rotation axis; the node's own X, Y or Z.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RotateAxis {
    X,
    Y,
    Z,
}

impl RotateAxis {
    fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RotateUnsupported {
    SelectedPivot,
    Gimbal,
    NumericalAngle,
}

impl fmt::Display for RotateUnsupported {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::SelectedPivot => "the selected rotate pivot must be zero",
            Self::Gimbal => "rotation is too close to gimbal lock",
            Self::NumericalAngle => "rotation cannot be represented accurately",
        })
    }
}

/// Baseline rotation of one node, as declared channel values and internal radians.
#[derive(Clone)]
pub struct ObjectRotateFrame {
    declared: [f64; 3],
    radians: [f64; 3],
    absent_leaf: [bool; 3],
    units: Units,
}

impl fmt::Debug for ObjectRotateFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObjectRotateFrame").finish_non_exhaustive()
    }
}

fn rotation_matrix([x, y, z]: [f64; 3]) -> DMat3 {
    DMat3::from_rotation_z(z) * DMat3::from_rotation_y(y) * DMat3::from_rotation_x(x)
}

// Shift `value` by whole turns to the representative nearest `base`.
fn unwrap_toward(value: f64, base: f64) -> f64 {
    value + TAU * ((base - value) / TAU).round()
}

impl ObjectRotateFrame {
    // Maya binary files omit default (zero) rotate leaves and the encoder cannot insert them.
    // An X ring changes rotateX only; Y and Z rings can change all three channels.
    pub fn require_leaves(&self, axis: RotateAxis) -> Result<(), NativeError> {
        let needed = if axis == RotateAxis::X { 0..1 } else { 0..3 };
        for component in needed {
            if self.absent_leaf[component] {
                return Err(NativeError::AbsentChannelLeaf {
                    channel: ChannelLeaf::Rotate,
                    component,
                });
            }
        }
        Ok(())
    }

    /// Declared rotate channel values after turning about the node's own `axis` by `degrees`.
    pub fn rotation(&self, axis: RotateAxis, degrees: f64) -> Result<[f64; 3], NativeError> {
        if !degrees.is_finite() {
            return Err(NativeError::NonFinite);
        }
        if degrees == 0.0 {
            return Ok(self.declared);
        }
        let delta = degrees.to_radians();
        let refused = NativeError::UnsupportedRotate(RotateUnsupported::NumericalAngle);
        if !delta.is_finite() || delta.abs() > MAX_ANGLE_RADIANS {
            return Err(refused);
        }
        let mut radians = if axis == RotateAxis::X {
            let mut radians = self.radians;
            radians[0] += delta;
            radians
        } else {
            self.compose(axis, delta)?
        };
        let intended = axis.index();
        let mut resolution: f64 = 0.0;
        for (component, (value, &base)) in radians.iter_mut().zip(&self.radians).enumerate() {
            if component != intended && (*value - base).abs() <= SNAP_TOLERANCE * (base.abs() + 1.0)
            {
                *value = base;
            }
            if value.to_bits() != base.to_bits() {
                resolution = resolution.max(ulp_bound(*value) + ulp_bound(base));
            }
        }
        if !radians.iter().all(|value| value.is_finite())
            || resolution > MIN_RESOLUTION * delta.abs()
        {
            return Err(refused);
        }
        let mut declared = self.declared;
        for component in 0..3 {
            if radians[component].to_bits() != self.radians[component].to_bits() {
                declared[component] = Storage::Radians.declared(radians[component], &self.units)?;
            }
        }
        Ok(declared)
    }

    // R_new = R_old * R_axis(delta) decomposed back to Maya xyz Euler angles (R = Rz Ry Rx),
    // choosing the representation nearest the baseline channels.
    fn compose(&self, axis: RotateAxis, delta: f64) -> Result<[f64; 3], NativeError> {
        let step = match axis {
            RotateAxis::Y => DMat3::from_rotation_y(delta),
            _ => DMat3::from_rotation_z(delta),
        };
        let target = rotation_matrix(self.radians) * step;
        let pitch = (-target.x_axis.z).clamp(-1.0, 1.0).asin();
        if !target.is_finite() || pitch.cos().abs() < MIN_COS_PITCH {
            return Err(NativeError::UnsupportedRotate(RotateUnsupported::Gimbal));
        }
        let roll = target.y_axis.z.atan2(target.z_axis.z);
        let yaw = target.x_axis.y.atan2(target.x_axis.x);
        let rebuilt = rotation_matrix([roll, pitch, yaw]);
        let drift = (rebuilt - target).to_cols_array().map(f64::abs);
        if drift.into_iter().fold(0.0, f64::max) > ROUNDTRIP_EPSILON {
            return Err(NativeError::UnsupportedRotate(
                RotateUnsupported::NumericalAngle,
            ));
        }
        let mut best: Option<([f64; 3], f64)> = None;
        for branch in [[roll, pitch, yaw], [roll + PI, PI - pitch, yaw + PI]] {
            let candidate: [f64; 3] = std::array::from_fn(|component| {
                unwrap_toward(branch[component], self.radians[component])
            });
            let distance: f64 = (0..3)
                .map(|component| (candidate[component] - self.radians[component]).abs())
                .sum();
            if best.is_none_or(|(_, nearest)| distance < nearest) {
                best = Some((candidate, distance));
            }
        }
        best.map(|(angles, _)| angles)
            .ok_or(NativeError::UnsupportedRotate(RotateUnsupported::Gimbal))
    }
}

impl NativeScene {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "wired into History::begin_rotate in the next commit"
        )
    )]
    pub(crate) fn begin_object_rotation(
        &self,
        target: &TransformTarget,
        axis: RotateAxis,
    ) -> Result<(ObjectRotateFrame, ChannelBaseline), NativeError> {
        if target.path.len() > MAX_TRANSFORM_DEPTH {
            return Err(NativeError::ResourceLimit);
        }
        let selected = self.resolve_transform_path(target.node, &target.path)?;
        self.editable(target.node)?;
        self.units.factors()?;
        let transform = self.internal_transform(target.node)?;
        if transform
            .rotate_pivot_cm
            .into_iter()
            .any(|value| value != 0.0)
        {
            return Err(NativeError::UnsupportedRotate(
                RotateUnsupported::SelectedPivot,
            ));
        }
        let mut absent_leaf = [false; 3];
        if let Some(source) = &self.source {
            if source.format != SourceFormat::Binary {
                return Err(NativeError::ReadOnlyFormat);
            }
            for (component, scalar) in selected.field(TransformField::Rotation).iter().enumerate() {
                let origin = match &scalar.origin {
                    Some(origin @ ScalarOrigin::Binary { .. }) => origin,
                    None => {
                        absent_leaf[component] = true;
                        continue;
                    }
                    #[cfg(test)]
                    Some(_) => return Err(NativeError::UnsupportedEdit),
                };
                source.original(
                    origin,
                    selected.transform_id,
                    TransformField::Rotation,
                    component,
                )?;
            }
        }
        let frame = ObjectRotateFrame {
            declared: std::array::from_fn(|component| selected.channels[3 + component].value),
            radians: transform.rotation_radians,
            absent_leaf,
            units: self.units.clone(),
        };
        frame.require_leaves(axis)?;
        Ok((
            frame,
            self.channel_baseline(selected, TransformField::Rotation),
        ))
    }
}

#[cfg(test)]
mod tests;
