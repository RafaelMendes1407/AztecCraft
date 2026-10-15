use super::*;
use crate::maya_binary::{container::Chunk, semantic::mesh::corner};

pub(super) fn tweak(
    container: &Container,
    chunk: &Chunk,
) -> Result<(usize, Range<usize>), BinaryError> {
    let range = chunk.payload_range();
    let bytes = &container.source()[range.clone()];
    let end = bytes
        .iter()
        .position(|b| *b == 0)
        .ok_or_else(|| fail(ErrorKind::Malformed, range.start))?;
    let key = std::str::from_utf8(&bytes[..end])
        .map_err(|_| fail(ErrorKind::Unsupported, range.start))?;
    let Some(pair) = key
        .strip_prefix("pt[")
        .and_then(|key| key.strip_suffix(']'))
        .and_then(|key| key.split_once(':'))
    else {
        return Err(fail(ErrorKind::Unsupported, range.start));
    };
    let number = |value: &str| -> Result<usize, BinaryError> {
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(fail(ErrorKind::Unsupported, range.start));
        }
        value
            .parse()
            .map_err(|_| fail(ErrorKind::Malformed, range.start))
    };
    let start = number(pair.0)?;
    let stop = number(pair.1)?;
    let count = stop
        .checked_sub(start)
        .and_then(|n| n.checked_add(1))
        .ok_or_else(|| fail(ErrorKind::Malformed, range.start))?;
    let value = range.start.checked_add(end + 2).ok_or_else(resource)?;
    let length = count.checked_mul(12).ok_or_else(resource)?;
    if bytes.get(end + 1) != Some(&0x20) || value.checked_add(length) != Some(range.end) {
        return Err(fail(ErrorKind::Malformed, range.start));
    }
    Ok((start, value..range.end))
}

pub(super) fn construct(
    container: &Container,
    facts: &WireFacts<'_>,
    binding: &Binding,
    budget: &mut Budget,
) -> Result<NativeMesh, BinaryError> {
    let record = binding
        .mesh
        .and_then(|i| facts.records.get(i))
        .ok_or_else(|| {
            fail(
                ErrorKind::Unsupported,
                container.chunk(binding.group).unwrap().header_range().start,
            )
        })?;
    let Data::Mesh(mesh) = &record.data else {
        unreachable!()
    };
    let mut base = budget.vector::<[f32; 3]>(mesh.positions.count / 3)?;
    for words in container.source()[mesh.positions.range.clone()]
        .as_chunks::<12>()
        .0
    {
        base.push(std::array::from_fn(|component| {
            f32::from_be_bytes(words[component * 4..component * 4 + 4].try_into().unwrap())
        }));
    }
    let mut edges = budget.vector(mesh.endpoints.count / 2)?;
    let mut endpoints = mesh.endpoints.values(container.source());
    while let Some(first) = endpoints.next() {
        edges.push(NativeEdge {
            endpoints: [
                (first & 0x7fff_ffff) as usize,
                endpoints.next().unwrap() as usize,
            ],
            smooth: None,
        });
    }
    let faces_count = mesh
        .corners
        .values(container.source())
        .filter(|word| word >> 24 & 0x60 == 0x60)
        .count();
    let mut faces = budget.vector(faces_count)?;
    let words = container.source()[mesh.corners.range.clone()]
        .as_chunks::<4>()
        .0;
    let mut position = 0;
    while position < words.len() {
        let count = words[position..]
            .iter()
            .position(|word| u32::from_be_bytes(*word) >> 24 & 0x60 == 0x60)
            .unwrap()
            + 1;
        let mut outer = budget.vector(count)?;
        for (index, word) in words[position..position + count].iter().enumerate() {
            let (edge, reversed, _) = corner(
                u32::from_be_bytes(*word),
                mesh.corners.range.start + (position + index) * 4,
            )?;
            outer.push(DirectedEdge { edge, reversed });
        }
        faces.push(NativeFace {
            outer,
            holes: Vec::new(),
        });
        position += count;
    }
    let count = children(container, binding.group)
        .filter(|chunk| chunk.tag() == *b"FLT3")
        .try_fold(0usize, |count, chunk| {
            count
                .checked_add(tweak(container, chunk)?.1.len() / 12)
                .ok_or_else(resource)
        })?;
    let mut offsets = budget.vector(count)?;
    for chunk in children(container, binding.group).filter(|chunk| chunk.tag() == *b"FLT3") {
        let (start, range) = tweak(container, chunk)?;
        for (index, words) in container.source()[range]
            .as_chunks::<12>()
            .0
            .iter()
            .enumerate()
        {
            offsets.push(PointOffset {
                vertex: start.checked_add(index).ok_or_else(resource)?,
                delta_cm: std::array::from_fn(|component| {
                    f32::from_be_bytes(words[component * 4..component * 4 + 4].try_into().unwrap())
                }),
            });
        }
    }
    offsets.sort_unstable_by_key(|offset| offset.vertex);
    if offsets
        .windows(2)
        .any(|pair| pair[0].vertex == pair[1].vertex)
    {
        return Err(fail(ErrorKind::Malformed, mesh.key.start));
    }
    NativeMesh::from_binary_parts(base, edges, faces, offsets)
        .map_err(|error| canonical(error, Some(mesh.key.start)))
}
