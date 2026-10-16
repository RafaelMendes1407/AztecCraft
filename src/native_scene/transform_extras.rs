//! Optional transform attributes beyond translate, rotate, scale and the two pivots:
//! rotateOrder (`ro`), rotateAxis (`ra`), shear (`sh`), rotatePivotTranslate (`rpt`) and
//! scalePivotTranslate (`spt`). `None` means the source did not state the attribute.
//! `opm` and `it` are deliberately absent (Slice A2); sources that carry them stay refused.
use super::{NativeError, Units, source::Storage};
use glam::{DMat4, DVec3};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TransformExtras {
    /// 0 xyz, 1 yzx, 2 zxy, 3 xzy, 4 yxz, 5 zyx.
    pub rotate_order: Option<u8>,
    /// Declared in the scene's angular unit.
    pub rotate_axis: Option<[f64; 3]>,
    /// `[xy, xz, yz]`, unitless.
    pub shear: Option<[f64; 3]>,
    /// Declared in the scene's linear unit.
    pub rotate_pivot_translate: Option<[f64; 3]>,
    pub scale_pivot_translate: Option<[f64; 3]>,
}

fn zero(value: Option<[f64; 3]>) -> bool {
    value.is_none_or(|v| v.iter().all(|c| *c == 0.0))
}

impl TransformExtras {
    /// True when every composed value equals the absent default. `Some(default)` counts.
    pub fn is_default(&self) -> bool {
        self.rotate_order.is_none_or(|order| order == 0)
            && zero(self.rotate_axis)
            && zero(self.shear)
            && zero(self.rotate_pivot_translate)
            && zero(self.scale_pivot_translate)
    }

    pub(crate) fn validate(&self) -> Result<(), NativeError> {
        if self.rotate_order.is_some_and(|order| order > 5) {
            return Err(NativeError::InvalidTransformExtras);
        }
        let finite = [
            self.rotate_axis,
            self.shear,
            self.rotate_pivot_translate,
            self.scale_pivot_translate,
        ]
        .into_iter()
        .flatten()
        .all(|v| v.iter().all(|c| c.is_finite()));
        if finite {
            Ok(())
        } else {
            Err(NativeError::NonFinite)
        }
    }

    /// `None` when all values are default, so callers keep the unchanged matrix path.
    pub(crate) fn internal(&self, units: &Units) -> Result<Option<InternalExtras>, NativeError> {
        self.validate()?;
        if self.is_default() {
            return Ok(None);
        }
        let convert = |value: Option<[f64; 3]>, storage: Storage| {
            let mut out = [0.0; 3];
            for (slot, component) in out.iter_mut().zip(value.unwrap_or_default()) {
                *slot = storage.internal(component, units)?;
            }
            Ok::<_, NativeError>(out)
        };
        Ok(Some(InternalExtras {
            rotate_order: self.rotate_order.unwrap_or(0),
            rotate_axis_radians: convert(self.rotate_axis, Storage::Radians)?,
            shear: convert(self.shear, Storage::Unitless)?,
            rotate_pivot_translate_cm: convert(self.rotate_pivot_translate, Storage::Centimeters)?,
            scale_pivot_translate_cm: convert(self.scale_pivot_translate, Storage::Centimeters)?,
        }))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InternalExtras {
    pub rotate_order: u8,
    pub rotate_axis_radians: [f64; 3],
    pub shear: [f64; 3],
    pub rotate_pivot_translate_cm: [f64; 3],
    pub scale_pivot_translate_cm: [f64; 3],
}

/// Axis indices applied first to last for each rotateOrder value; `None` outside 0 to 5.
fn order_axes(order: u8) -> Option<[usize; 3]> {
    Some(match order {
        0 => [0, 1, 2],
        1 => [1, 2, 0],
        2 => [2, 0, 1],
        3 => [0, 2, 1],
        4 => [1, 0, 2],
        5 => [2, 1, 0],
        _ => return None,
    })
}

fn axis_rotation(axis: usize, radians: f64) -> DMat4 {
    match axis {
        0 => DMat4::from_rotation_x(radians),
        1 => DMat4::from_rotation_y(radians),
        _ => DMat4::from_rotation_z(radians),
    }
}

/// Euler rotation for `order`: the first axis in the order is applied first (rightmost column factor).
/// `None` for an order validation would have rejected, so callers refuse instead of clamping.
pub(crate) fn rotation_matrix(order: u8, radians: [f64; 3]) -> Option<DMat4> {
    let axes = order_axes(order)?;
    Some(
        axis_rotation(axes[2], radians[axes[2]])
            * axis_rotation(axes[1], radians[axes[1]])
            * axis_rotation(axes[0], radians[axes[0]]),
    )
}

/// The 2026 transform page documents the matrix layout (row-vector [1][0]=xy, [2][0]=xz,
/// [2][1]=yz; the column form built here is its transpose). PROVISIONAL: only the
/// attribute's [xy, xz, yz] component order is unconfirmed against a Maya reference scene.
fn shear_matrix(shear: [f64; 3]) -> DMat4 {
    let [xy, xz, yz] = shear;
    DMat4::from_cols_array(&[
        1.0, 0.0, 0.0, 0.0, //
        xy, 1.0, 0.0, 0.0, //
        xz, yz, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ])
}

impl InternalExtras {
    /// RA as xyz Euler (Rz * Ry * Rx). The single source for the matrix and the handle frame.
    pub(crate) fn rotate_axis_matrix(&self) -> DMat4 {
        let [ax, ay, az] = self.rotate_axis_radians;
        DMat4::from_rotation_z(az) * DMat4::from_rotation_y(ay) * DMat4::from_rotation_x(ax)
    }

    /// T * RT * RP * R(ro) * RA * RP^-1 * ST * SP * SH * S * SP^-1 (column form).
    pub(crate) fn matrix(
        &self,
        translate: DVec3,
        rotation: [f64; 3],
        scale: DVec3,
        rp: DVec3,
        sp: DVec3,
    ) -> Option<DMat4> {
        // PROVISIONAL: RA is composed as xyz Euler, but the 2026 transform page says
        // rotateAxis is stored as a quaternion; unconfirmed against a Maya reference scene.
        Some(
            DMat4::from_translation(translate)
                * DMat4::from_translation(DVec3::from_array(self.rotate_pivot_translate_cm))
                * DMat4::from_translation(rp)
                * rotation_matrix(self.rotate_order, rotation)?
                * self.rotate_axis_matrix()
                * DMat4::from_translation(-rp)
                * DMat4::from_translation(DVec3::from_array(self.scale_pivot_translate_cm))
                * DMat4::from_translation(sp)
                * shear_matrix(self.shear)
                * DMat4::from_scale(scale)
                * DMat4::from_translation(-sp),
        )
    }
}

#[cfg(test)]
pub(crate) mod tests;
