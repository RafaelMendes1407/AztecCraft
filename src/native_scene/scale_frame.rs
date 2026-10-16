use super::move_translation::ChannelBaseline;
use super::*;

/// What an object-space scale drag grabs: one local axis, or the center box for all three.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScaleHandle {
    X,
    Y,
    Z,
    Uniform,
}

impl ScaleHandle {
    fn components(self) -> std::ops::Range<usize> {
        match self {
            Self::X => 0..1,
            Self::Y => 1..2,
            Self::Z => 2..3,
            Self::Uniform => 0..3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScaleUnsupported {
    SelectedPivot,
    InvalidFactor,
    Unrepresentable,
}

impl fmt::Display for ScaleUnsupported {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::SelectedPivot => "the selected scale pivot must be zero",
            Self::InvalidFactor => "scale factors must be finite and greater than zero",
            Self::Unrepresentable => "scaled channel overflows or underflows",
        })
    }
}

/// Baseline scale of one node; scale channels are unitless, so declared equals internal.
#[derive(Clone, Debug)]
pub struct ObjectScaleFrame {
    scale: [f64; 3],
    absent_leaf: [bool; 3],
}

impl ObjectScaleFrame {
    // Maya binary files omit default (one) scale leaves and the encoder cannot insert them.
    pub fn require_leaves(&self, handle: ScaleHandle) -> Result<(), NativeError> {
        for component in handle.components() {
            if self.absent_leaf[component] {
                return Err(NativeError::AbsentChannelLeaf {
                    channel: ChannelLeaf::Scale,
                    component,
                });
            }
        }
        Ok(())
    }

    /// Scale channel values after multiplying the handle's channels by `factor`.
    pub fn scale(&self, handle: ScaleHandle, factor: f64) -> Result<[f64; 3], NativeError> {
        if !factor.is_finite() {
            return Err(NativeError::NonFinite);
        }
        if factor <= 0.0 {
            return Err(NativeError::UnsupportedScale(
                ScaleUnsupported::InvalidFactor,
            ));
        }
        let mut scale = self.scale;
        for component in handle.components() {
            let scaled = scale[component] * factor;
            if !scaled.is_finite() || (scaled == 0.0 && scale[component] != 0.0) {
                return Err(NativeError::UnsupportedScale(
                    ScaleUnsupported::Unrepresentable,
                ));
            }
            scale[component] = scaled;
        }
        Ok(scale)
    }
}

impl NativeScene {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "wired into History::begin_scale in the next commit"
        )
    )]
    pub(crate) fn begin_object_scale(
        &self,
        target: &TransformTarget,
        handle: ScaleHandle,
    ) -> Result<(ObjectScaleFrame, ChannelBaseline), NativeError> {
        if target.path.len() > MAX_TRANSFORM_DEPTH {
            return Err(NativeError::ResourceLimit);
        }
        let selected = self.resolve_transform_path(target.node, &target.path)?;
        self.editable(target.node)?;
        let transform = self.internal_transform(target.node)?;
        if transform
            .scale_pivot_cm
            .into_iter()
            .any(|value| value != 0.0)
        {
            return Err(NativeError::UnsupportedScale(
                ScaleUnsupported::SelectedPivot,
            ));
        }
        let mut absent_leaf = [false; 3];
        if let Some(source) = &self.source {
            if source.format != SourceFormat::Binary {
                return Err(NativeError::ReadOnlyFormat);
            }
            for (component, scalar) in selected.field(TransformField::Scale).iter().enumerate() {
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
                    TransformField::Scale,
                    component,
                )?;
            }
        }
        let frame = ObjectScaleFrame {
            scale: std::array::from_fn(|component| selected.channels[6 + component].value),
            absent_leaf,
        };
        frame.require_leaves(handle)?;
        Ok((
            frame,
            self.channel_baseline(selected, TransformField::Scale),
        ))
    }
}

#[cfg(test)]
mod tests;
