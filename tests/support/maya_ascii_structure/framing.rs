const MAX_BYTES: usize = 16 * 1024 * 1024;
const HEADER_LINES: [&str; 4] = [
    "//Maya ASCII 2026 scene",
    "requires maya \"2026\";",
    "currentUnit -l centimeter -a degree -t film;",
    "fileInfo \"AztecCraft\" \"1\";",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Header,
    Transform,
    Mesh,
    Uuid,
    Channel,
    Array,
    Lambert,
    Color,
    Group,
    Connection,
}

fn next_line<'a>(
    lines: &mut impl Iterator<Item = &'a str>,
    offset: &mut usize,
) -> Option<(usize, &'a str)> {
    let line = lines.next()?;
    let start = *offset;
    *offset += line.len() + 1;
    Some((start, line))
}

pub fn frame(bytes: &[u8]) -> Result<(Vec<Kind>, usize), String> {
    if bytes.len() > MAX_BYTES {
        return Err("input byte limit".into());
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "non-UTF8 input")?;
    if !text.ends_with('\n') || text.contains('\r') {
        return Err("canonical LF framing".into());
    }
    let mut lines = text.split_terminator('\n');
    let mut offset = 0;
    let mut kinds = Vec::new();
    for (index, expected) in HEADER_LINES.iter().enumerate() {
        let (_, line) = next_line(&mut lines, &mut offset).ok_or("short header")?;
        if line != *expected {
            return Err("noncanonical header line".into());
        }
        if index != 0 {
            kinds.push(Kind::Header);
        }
    }
    let mut boundary = bytes.len();
    let mut tail = false;
    while let Some((start, line)) = next_line(&mut lines, &mut offset) {
        if line.contains("//") || line.is_empty() {
            return Err("comment or blank body line".into());
        }
        let kind = if line.starts_with("\tsetAttr -s ") {
            if tail || line.contains(';') {
                return Err("array outside geometry or fused header".into());
            }
            let count = line
                .strip_prefix("\tsetAttr -s ")
                .and_then(|rest| rest.split_once(' '))
                .and_then(|(count, _)| count.parse::<usize>().ok())
                .ok_or("array count")?;
            if count == 0 || count > i32::MAX as usize || count > bytes.len() {
                return Err("array count limit".into());
            }
            let face = line.contains("\".fc[0:");
            if !(line.contains("\".vt[0:") || line.contains("\".ed[0:") || face)
                || (face && !line.ends_with("]\" -type \"polyFaces\""))
                || (!face && !line.ends_with("]\""))
            {
                return Err("array header".into());
            }
            for _ in 0..count {
                let (_, row) = next_line(&mut lines, &mut offset).ok_or("short array")?;
                if !row.starts_with(if face { "\tf " } else { "\t" })
                    || row.contains(';')
                    || row.contains("//")
                {
                    return Err("array row framing".into());
                }
            }
            if next_line(&mut lines, &mut offset).map(|(_, line)| line) != Some(";") {
                return Err("array terminator".into());
            }
            Kind::Array
        } else {
            if !line.ends_with(';') || line.bytes().filter(|c| *c == b';').count() != 1 {
                return Err("single statement framing".into());
            }
            if line.starts_with("createNode transform ") {
                Kind::Transform
            } else if line.starts_with("createNode mesh ") {
                Kind::Mesh
            } else if line.starts_with("\trename -uid ") {
                Kind::Uuid
            } else if line.starts_with("\tsetAttr \".c\" ") {
                Kind::Color
            } else if line.starts_with("\tsetAttr ") {
                Kind::Channel
            } else if line.starts_with("createNode lambert ") {
                Kind::Lambert
            } else if line.starts_with("createNode shadingEngine ") {
                Kind::Group
            } else if line.starts_with("connectAttr ") {
                Kind::Connection
            } else {
                return Err("unknown body statement".into());
            }
        };
        if matches!(kind, Kind::Lambert | Kind::Connection) && boundary == bytes.len() {
            boundary = start;
        }
        tail |= matches!(
            kind,
            Kind::Lambert | Kind::Color | Kind::Group | Kind::Connection
        );
        if tail
            && matches!(
                kind,
                Kind::Transform | Kind::Mesh | Kind::Uuid | Kind::Channel
            )
        {
            return Err("geometry after material tail".into());
        }
        kinds.push(kind);
    }
    Ok((kinds, boundary))
}
