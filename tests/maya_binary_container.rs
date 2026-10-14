use azteccraft::maya_binary::container::{
    self, Container, ErrorKind, MAX_BYTES, MAX_CHUNKS, MAX_DEPTH,
};
use std::{
    fs::File,
    io::Read,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

fn record(tag: [u8; 4], auxiliary: u32, payload: &[u8], alignment: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(tag);
    bytes.extend(auxiliary.to_be_bytes());
    bytes.extend((payload.len() as u64).to_be_bytes());
    bytes.extend(payload);
    bytes.resize(16 + payload.len().next_multiple_of(alignment), 0xa5);
    bytes
}

fn group(tag: [u8; 4], form: [u8; 4], children: &[u8], alignment: usize) -> Vec<u8> {
    let mut payload = form.to_vec();
    payload.extend(children);
    record(tag, 0xfedc_ba98, &payload, alignment)
}

fn root(children: &[u8]) -> Vec<u8> {
    group(*b"FOR8", *b"Maya", children, 1)
}

fn parsed(bytes: Vec<u8>) -> Container {
    container::inspect(Arc::from(bytes)).unwrap()
}

fn assert_partition(container: &Container) {
    let chunks = container.chunks();
    assert_eq!(chunks[0].header_range().start, 0);
    assert_eq!(chunks[0].padding_range().end, container.source().len());
    for (index, chunk) in chunks.iter().enumerate() {
        assert_eq!(chunk.header_range().end, chunk.payload_range().start);
        assert_eq!(chunk.payload_range().end, chunk.padding_range().start);
        if chunk.form().is_some() {
            let mut cursor = chunk.payload_range().start + 4;
            for child in chunks.iter().filter(|child| child.parent() == Some(index)) {
                assert_eq!(child.header_range().start, cursor);
                cursor = child.padding_range().end;
            }
            assert_eq!(cursor, chunk.payload_range().end);
        } else {
            assert!(!chunks.iter().any(|child| child.parent() == Some(index)));
        }
    }
}

#[test]
fn synthetic_unknown_leaf_and_padding_preserve_the_original_arc_and_header_bits() {
    let opaque = b"unexecuted script bytes\0\xff";
    let leaf = record([0xff, 0x01, 0x00, 0x7f], 0x8123_4567, opaque, 4);
    let source: Arc<[u8]> = Arc::from(root(&leaf));
    let container = container::inspect(source.clone()).unwrap();
    assert!(Arc::ptr_eq(&source, container.source()));
    assert_eq!(container.chunks().len(), 2);
    let chunk = container.chunk(1).unwrap();
    assert_eq!(chunk.tag(), [0xff, 0x01, 0x00, 0x7f]);
    assert_eq!(chunk.auxiliary(), 0x8123_4567);
    assert_eq!(container.payload(1), Some(opaque.as_slice()));
    assert!(
        container.source()[chunk.padding_range()]
            .iter()
            .all(|byte| *byte == 0xa5)
    );
    assert!(container.chunk(2).is_none());
    assert!(container.payload(usize::MAX).is_none());
    let debug = format!("{container:?}");
    assert!(!debug.contains(&format!("{:?}", container.source())));
    assert!(debug.contains("source_len"));
    assert!(debug.contains("chunk_count: 2"));
    assert_partition(&container);
}

#[test]
fn synthetic_cons_children_align_to_four_while_connection_leaves_align_to_eight() {
    let mut leaves = record(*b"CWFL", 7, b"opaque connection", 8);
    leaves.extend(record(*b"RELA", 9, b"relation", 8));
    let conn = group(*b"FOR8", *b"CONN", &leaves, 4);
    assert_eq!(u64::from_be_bytes(conn[8..16].try_into().unwrap()) % 8, 4);
    let cons = group(*b"LIS8", *b"CONS", &conn, 4);
    let mut children = cons;
    children.extend(group(
        *b"FOR8",
        *b"HEAD",
        &record(*b"VERS", 0, b"synthetic", 8),
        4,
    ));
    let container = parsed(root(&children));
    assert_eq!(container.chunks().len(), 7);
    assert_eq!(container.chunk(1).unwrap().form(), Some(*b"CONS"));
    assert_eq!(container.chunk(2).unwrap().form(), Some(*b"CONN"));
    assert_eq!(container.chunk(5).unwrap().form(), Some(*b"HEAD"));
    assert_partition(&container);
}

#[test]
fn synthetic_unknown_group_layouts_refuse_without_reinterpreting_them_as_leaves() {
    assert_eq!(
        container::inspect(Arc::from(b"FOR4\0\0\0\x04Maya".as_slice()))
            .unwrap_err()
            .kind(),
        ErrorKind::Unsupported
    );
    for (tag, form) in [
        (*b"FOR8", *b"NEW!"),
        (*b"LIS8", *b"HEAD"),
        (*b"CAT8", *b"HEAD"),
        (*b"FOR4", *b"HEAD"),
    ] {
        let error = container::inspect(Arc::from(root(&group(tag, form, &[], 4)))).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Unsupported);
        assert_eq!(error.offset(), 20);
    }
    let numeric = group(*b"FOR8", [0x00, 0x11, 0x5c, 0x85], &[], 4);
    let container = parsed(root(&numeric));
    assert_eq!(
        container.chunk(1).unwrap().form(),
        Some([0x00, 0x11, 0x5c, 0x85])
    );
}

#[test]
fn synthetic_malformed_nested_child_never_returns_a_successful_prefix() {
    let mut bad = record(*b"VERS", 0, b"a", 8);
    bad[8..16].copy_from_slice(&u64::MAX.to_be_bytes());
    let bytes = root(&group(*b"FOR8", *b"HEAD", &bad, 4));
    let error = container::inspect(Arc::from(bytes)).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Overflow);
    assert_eq!(error.offset(), 40);
    for residual in 1..16 {
        assert_eq!(
            container::inspect(Arc::from(root(&vec![0; residual])))
                .unwrap_err()
                .kind(),
            ErrorKind::Truncated
        );
    }
}

#[test]
fn synthetic_truncated_forms_payloads_padding_and_trailing_bytes_are_rejected() {
    let valid = root(&record(*b"LEAF", 0, b"123", 4));
    for end in 0..valid.len() {
        assert!(container::inspect(Arc::from(valid[..end].to_vec())).is_err());
    }
    let mut trailing = valid.clone();
    trailing.push(0);
    assert_eq!(
        container::inspect(Arc::from(trailing)).unwrap_err().kind(),
        ErrorKind::TrailingBytes
    );
    let mut truncated = valid;
    truncated.pop();
    let size = (truncated.len() - 16) as u64;
    truncated[8..16].copy_from_slice(&size.to_be_bytes());
    assert_eq!(
        container::inspect(Arc::from(truncated)).unwrap_err().kind(),
        ErrorKind::Truncated
    );
    assert_eq!(
        container::inspect(Arc::from(record(*b"FOR8", 0, b"May", 1)))
            .unwrap_err()
            .kind(),
        ErrorKind::Truncated
    );
}

fn nested_groups(total: usize) -> Vec<u8> {
    let mut children = Vec::new();
    for level in (1..total).rev() {
        children = group(
            *b"FOR8",
            *b"HEAD",
            &children,
            if level == 1 { 4 } else { 8 },
        );
    }
    root(&children)
}

#[test]
fn synthetic_byte_chunk_and_depth_resource_limits_include_the_root() {
    assert_eq!(
        container::inspect(Arc::from(vec![0; MAX_BYTES + 1]))
            .unwrap_err()
            .kind(),
        ErrorKind::ByteLimit
    );
    let bytes = root(&record(*b"LEAF", 0, &vec![0; MAX_BYTES - 36], 4));
    assert_eq!(bytes.len(), MAX_BYTES);
    assert_eq!(parsed(bytes).source().len(), MAX_BYTES);
    let leaf = record(*b"LEAF", 0, &[], 4);
    let mut children = leaf.repeat(MAX_CHUNKS - 1);
    assert_eq!(parsed(root(&children)).chunks().len(), MAX_CHUNKS);
    children.extend(leaf);
    assert_eq!(
        container::inspect(Arc::from(root(&children)))
            .unwrap_err()
            .kind(),
        ErrorKind::ChunkLimit
    );
    assert_eq!(parsed(nested_groups(MAX_DEPTH)).chunks().len(), MAX_DEPTH);
    assert_eq!(
        container::inspect(Arc::from(nested_groups(MAX_DEPTH + 1)))
            .unwrap_err()
            .kind(),
        ErrorKind::DepthLimit
    );
}

#[derive(serde::Deserialize)]
struct Fixture {
    relative_path: String,
    bytes: usize,
    total_chunks_including_root: usize,
    group_chunks_including_root: usize,
    leaf_chunks: usize,
    cons_groups: usize,
    conn_groups: usize,
    trailing_head_offset: usize,
    trailing_head_size: usize,
}

fn bounded_read(path: &Path) -> Vec<u8> {
    let mut bytes = Vec::new();
    File::open(path)
        .expect("private fixture input must exist")
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .expect("private fixture input must be readable");
    assert!(
        bytes.len() <= MAX_BYTES,
        "private fixture input exceeds reader byte policy"
    );
    bytes
}

fn fixture_path(directory: &Path, relative: &Path) -> Option<PathBuf> {
    if !relative
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
    {
        return None;
    }
    let directory = directory.canonicalize().ok()?;
    let candidate = directory.join(relative).canonicalize().ok()?;
    candidate.starts_with(&directory).then_some(candidate)
}

#[cfg(unix)]
#[test]
fn private_fixture_paths_refuse_symlink_escape() {
    let directory = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let outside_file = outside.path().join("scene.mb");
    std::fs::write(&outside_file, root(&[])).unwrap();
    std::os::unix::fs::symlink(&outside_file, directory.path().join("link.mb")).unwrap();
    assert!(fixture_path(directory.path(), Path::new("link.mb")).is_none());
    let inside = directory.path().join("scene.mb");
    std::fs::write(&inside, root(&[])).unwrap();
    assert_eq!(
        fixture_path(directory.path(), Path::new("scene.mb")),
        Some(inside.canonicalize().unwrap())
    );
    assert!(fixture_path(directory.path(), Path::new("../scene.mb")).is_none());
}

#[test]
#[ignore = "requires explicit private manifest and separately verified SHA256 precheck"]
fn private_fixture_manifest_partitions_all_bytes() {
    let manifest_path = std::env::var_os("AZTECCRAFT_MB_FIXTURE_MANIFEST")
        .expect("explicit private manifest is required");
    let manifest_path = Path::new(&manifest_path);
    let fixtures: Vec<Fixture> = serde_json::from_slice(&bounded_read(manifest_path))
        .expect("private manifest must be valid");
    assert_eq!(fixtures.len(), 5);
    for fixture in fixtures {
        let relative = Path::new(&fixture.relative_path);
        let path = fixture_path(manifest_path.parent().unwrap(), relative)
            .expect("private fixture must resolve beneath manifest directory");
        let bytes = bounded_read(&path);
        assert_eq!(bytes.len(), fixture.bytes);
        let container = parsed(bytes);
        assert_partition(&container);
        let chunks = container.chunks();
        assert_eq!(chunks.len(), fixture.total_chunks_including_root);
        assert_eq!(
            chunks.iter().filter(|chunk| chunk.form().is_some()).count(),
            fixture.group_chunks_including_root
        );
        assert_eq!(
            chunks.iter().filter(|chunk| chunk.form().is_none()).count(),
            fixture.leaf_chunks
        );
        assert_eq!(
            chunks
                .iter()
                .filter(|chunk| chunk.tag() == *b"LIS8" && chunk.form() == Some(*b"CONS"))
                .count(),
            fixture.cons_groups
        );
        assert_eq!(
            chunks
                .iter()
                .filter(|chunk| chunk.tag() == *b"FOR8" && chunk.form() == Some(*b"CONN"))
                .count(),
            fixture.conn_groups
        );
        let trailing = chunks
            .iter()
            .rev()
            .find(|chunk| chunk.parent() == Some(0))
            .unwrap();
        assert_eq!(trailing.form(), Some(*b"HEAD"));
        assert_eq!(trailing.header_range().start, fixture.trailing_head_offset);
        assert_eq!(trailing.payload_range().len(), fixture.trailing_head_size);
        assert_eq!(trailing.padding_range().end, container.source().len());
    }
}
