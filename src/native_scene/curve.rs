use super::{GeometryViewFacts, NativeError, NativeId, SourceFormat, SourceRetention};
use std::{fmt, ops::Range};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurveForm {
    Open,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CurveSource {
    pub(super) owner: NativeId,
    pub(super) statement: Range<usize>,
}

#[derive(Clone)]
pub struct NativeCurveShape {
    pub(super) id: NativeId,
    pub(super) uuid: Option<Uuid>,
    pub(super) name: String,
    pub(super) control_vertices: Vec<[f64; 3]>,
    pub(super) knots: Vec<f64>,
    pub(super) source: CurveSource,
    pub(super) view: GeometryViewFacts,
}

impl PartialEq for NativeCurveShape {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.uuid == other.uuid
            && self.name == other.name
            && self.source == other.source
            && self.view == other.view
            && self.control_vertices.len() == other.control_vertices.len()
            && self.knots.len() == other.knots.len()
            && self
                .control_vertices
                .iter()
                .zip(&other.control_vertices)
                .all(|(a, b)| a.map(f64::to_bits) == b.map(f64::to_bits))
            && self
                .knots
                .iter()
                .zip(&other.knots)
                .all(|(a, b)| a.to_bits() == b.to_bits())
    }
}

impl fmt::Debug for NativeCurveShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeCurveShape")
            .field("id", &self.id)
            .field("cv_count", &self.control_vertices.len())
            .field("knot_count", &self.knots.len())
            .finish()
    }
}

impl NativeCurveShape {
    pub fn id(&self) -> NativeId {
        self.id
    }
    pub fn uuid(&self) -> Option<Uuid> {
        self.uuid
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn degree(&self) -> u32 {
        3
    }
    pub fn form(&self) -> CurveForm {
        CurveForm::Open
    }
    pub fn control_vertices(&self) -> &[[f64; 3]] {
        &self.control_vertices
    }
    pub fn knots(&self) -> &[f64] {
        &self.knots
    }
    pub fn domain(&self) -> Result<[f64; 2], NativeError> {
        let end = self
            .knots
            .len()
            .checked_sub(3)
            .ok_or(NativeError::InvalidCurve)?;
        Ok([
            *self.knots.get(2).ok_or(NativeError::InvalidCurve)?,
            *self.knots.get(end).ok_or(NativeError::InvalidCurve)?,
        ])
    }

    pub(super) fn validate(&self, source: Option<&SourceRetention>) -> Result<(), NativeError> {
        let source = source
            .filter(|s| s.format == SourceFormat::Ascii)
            .ok_or(NativeError::InvalidRetention)?;
        if self.source.owner != self.id
            || self.source.statement.start >= self.source.statement.end
            || source.bytes.get(self.source.statement.clone()).is_none()
            || !source.records.contains(&self.source.statement)
        {
            return Err(NativeError::InvalidRetention);
        }
        let count = self.control_vertices.len();
        if count < 4 || count.checked_add(2) != Some(self.knots.len()) {
            return Err(NativeError::InvalidCurve);
        }
        if self
            .control_vertices
            .iter()
            .flatten()
            .chain(&self.knots)
            .any(|v| !v.is_finite())
        {
            return Err(NativeError::NonFinite);
        }
        let [start, end] = self.domain()?;
        if start >= end
            || !(end - start).is_finite()
            || self.knots[..3].iter().any(|v| *v != start)
            || self.knots[self.knots.len() - 3..].iter().any(|v| *v != end)
            || self.knots[2..=self.knots.len() - 3]
                .windows(2)
                .any(|pair| pair[0] > pair[1] || !(pair[1] - pair[0]).is_finite())
        {
            return Err(NativeError::InvalidCurve);
        }
        if self.knots[2..=self.knots.len() - 3]
            .windows(2)
            .any(|pair| pair[0] == pair[1])
        {
            return Err(NativeError::UnsupportedView(
                "interior knot multiplicity (C0 or kinked curve)",
            ));
        }
        self.view.visible()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
