use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Integer,
    Enumeration,
    Double,
}

struct Definition {
    group: usize,
    key: Range<usize>,
    kind: Kind,
    offset: usize,
}

pub(super) struct Definitions<'a> {
    source: &'a [u8],
    entries: Vec<Definition>,
}

fn identifier(bytes: &[u8]) -> bool {
    bytes
        .first()
        .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_')
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'_')
}

fn reserved(bytes: &[u8]) -> bool {
    matches!(
        bytes,
        b"t" | b"translate"
            | b"tx"
            | b"ty"
            | b"tz"
            | b"translateX"
            | b"translateY"
            | b"translateZ"
            | b"r"
            | b"rotate"
            | b"rx"
            | b"ry"
            | b"rz"
            | b"rotateX"
            | b"rotateY"
            | b"rotateZ"
            | b"s"
            | b"scale"
            | b"sx"
            | b"sy"
            | b"sz"
            | b"scaleX"
            | b"scaleY"
            | b"scaleZ"
            | b"rp"
            | b"rotatePivot"
            | b"rpx"
            | b"rpy"
            | b"rpz"
            | b"rotatePivotX"
            | b"rotatePivotY"
            | b"rotatePivotZ"
            | b"sp"
            | b"scalePivot"
            | b"spx"
            | b"spy"
            | b"spz"
            | b"scalePivotX"
            | b"scalePivotY"
            | b"scalePivotZ"
            | b"ra"
            | b"rotateAxis"
            | b"rax"
            | b"ray"
            | b"raz"
            | b"rotateAxisX"
            | b"rotateAxisY"
            | b"rotateAxisZ"
            | b"rpt"
            | b"rotatePivotTranslate"
            | b"rptx"
            | b"rpty"
            | b"rptz"
            | b"rotatePivotTranslateX"
            | b"rotatePivotTranslateY"
            | b"rotatePivotTranslateZ"
            | b"spt"
            | b"scalePivotTranslate"
            | b"sptx"
            | b"spty"
            | b"sptz"
            | b"scalePivotTranslateX"
            | b"scalePivotTranslateY"
            | b"scalePivotTranslateZ"
            | b"v"
            | b"visibility"
            | b"io"
            | b"intermediateObject"
            | b"it"
            | b"inheritsTransform"
            | b"ro"
            | b"rotateOrder"
            | b"sh"
            | b"shear"
            | b"shxy"
            | b"shxz"
            | b"shyz"
            | b"shearXY"
            | b"shearXZ"
            | b"shearYZ"
            | b"opm"
            | b"offsetParentMatrix"
            | b"m"
            | b"matrix"
            | b"wm"
            | b"worldMatrix"
            | b"pm"
            | b"parentMatrix"
            | b"im"
            | b"inverseMatrix"
            | b"wim"
            | b"worldInverseMatrix"
            | b"pim"
            | b"parentInverseMatrix"
            | b"i"
            | b"inMesh"
            | b"ci"
            | b"cachedInMesh"
            | b"o"
            | b"outMesh"
            | b"worldMesh"
            | b"pt"
            | b"pnts"
            | b"cp"
            | b"controlPoints"
            | b"xValue"
            | b"yValue"
            | b"zValue"
            | b"px"
            | b"py"
            | b"pz"
            | b"pntx"
            | b"pnty"
            | b"pntz"
            | b"controlPointX"
            | b"controlPointY"
            | b"controlPointZ"
    )
}

fn definition(
    container: &Container,
    group: usize,
    chunk: &crate::maya_binary::container::Chunk,
) -> Result<Definition, BinaryError> {
    let range = chunk.payload_range();
    let bytes = &container.source()[range.clone()];
    let invalid = || fail(ErrorKind::Unsupported, range.start);
    let (kind, prefix): (Kind, &[u8]) = match bytes.get(..4) {
        Some(b"aLI1") => (
            Kind::Integer,
            b"aLI1\x00\xf8\x78\xc8\x00\x00\x00\x01\x00\x00\x00\x00",
        ),
        Some(b"aENM") => (
            Kind::Enumeration,
            b"aENM\x00\xf8\x02\x4e\x00\x00\x00\x01\x00\x00\x00\x00",
        ),
        Some(b"aDBL") => (
            Kind::Double,
            b"aDBL\x00\xf8\x00\x4e\x00\x00\x00\x01\x00\x00\x00\x00",
        ),
        _ => return Err(invalid()),
    };
    if !bytes.starts_with(prefix) {
        return Err(invalid());
    }
    let long_end = bytes[16..]
        .iter()
        .position(|b| *b == 0)
        .map(|n| n + 16)
        .ok_or_else(invalid)?;
    let short_start = long_end + 1;
    let short_end = bytes[short_start..]
        .iter()
        .position(|b| *b == 0)
        .map(|n| n + short_start)
        .ok_or_else(invalid)?;
    let key = &bytes[16..long_end];
    if !identifier(key) || key != &bytes[short_start..short_end] || reserved(key) {
        return Err(invalid());
    }
    let mut tail = short_end + 1;
    if kind == Kind::Enumeration {
        let end = bytes[tail..]
            .iter()
            .position(|b| *b == 0)
            .map(|n| n + tail)
            .ok_or_else(invalid)?;
        if end == tail || !bytes[tail..end].iter().all(|b| (0x20..=0x7e).contains(b)) {
            return Err(invalid());
        }
        tail = end + 1;
    }
    let Some(frame) = bytes.get(tail..) else {
        return Err(invalid());
    };
    if frame.len() != 32
        || frame[..4] != [0; 4]
        || &frame[20..24] != b"DBLE"
        || [4..12, 12..20, 24..32]
            .iter()
            .any(|range| !f64::from_be_bytes(frame[range.clone()].try_into().unwrap()).is_finite())
    {
        return Err(invalid());
    }
    Ok(Definition {
        group,
        key: range.start + 16..range.start + long_end,
        kind,
        offset: chunk.header_range().start,
    })
}

impl<'a> Definitions<'a> {
    pub fn inspect(
        container: &'a Container,
        bindings: &[Binding],
        budget: &mut Budget,
    ) -> Result<Self, BinaryError> {
        let count = container
            .chunks()
            .iter()
            .filter(|chunk| {
                chunk.tag() == *b"ATTR"
                    && chunk
                        .parent()
                        .is_some_and(|group| owner_index(bindings, group).is_some())
            })
            .count();
        let mut entries = budget.vector(count)?;
        for binding in bindings {
            for chunk in children(container, binding.group).filter(|chunk| chunk.tag() == *b"ATTR")
            {
                if binding.transform.is_none() {
                    return Err(fail(ErrorKind::Unsupported, chunk.payload_range().start));
                }
                entries.push(definition(container, binding.group, chunk)?);
            }
        }
        let source = container.source().as_ref();
        entries.sort_unstable_by(|a, b| {
            (a.group, &source[a.key.clone()]).cmp(&(b.group, &source[b.key.clone()]))
        });
        if entries.windows(2).any(|pair| {
            pair[0].group == pair[1].group
                && source[pair[0].key.clone()] == source[pair[1].key.clone()]
        }) {
            return Err(fail(ErrorKind::Malformed, 0));
        }
        Ok(Self { source, entries })
    }

    fn find(&self, group: usize, key: &[u8]) -> Option<&Definition> {
        self.entries
            .binary_search_by(|entry| {
                (entry.group, &self.source[entry.key.clone()]).cmp(&(group, key))
            })
            .ok()
            .map(|i| &self.entries[i])
    }

    pub fn state(
        &self,
        group: usize,
        key: &[u8],
        chunk: &crate::maya_binary::container::Chunk,
    ) -> Result<bool, BinaryError> {
        let Some(entry) = self.find(group, key) else {
            return Ok(false);
        };
        let range = chunk.payload_range();
        let bytes = &self.source[range.clone()];
        if chunk.header_range().start <= entry.offset {
            return Err(fail(ErrorKind::Unsupported, range.start));
        }
        let value = key.len() + 2;
        let valid = bytes.get(key.len() + 1) == Some(&0x24)
            && match entry.kind {
                Kind::Enumeration => chunk.tag() == *b"FLGS" && bytes.len() == value,
                Kind::Integer | Kind::Double => {
                    chunk.tag() == *b"DBLE"
                        && bytes.len() == value + 8
                        && f64::from_be_bytes(bytes[value..].try_into().unwrap()).is_finite()
                }
            };
        if !valid {
            return Err(fail(ErrorKind::Unsupported, range.start));
        }
        Ok(true)
    }

    pub fn output(&self, group: usize, key: &[u8]) -> bool {
        self.find(group, key)
            .is_some_and(|entry| matches!(entry.kind, Kind::Enumeration | Kind::Double))
    }
}
