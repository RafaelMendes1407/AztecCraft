#[path = "framing.rs"]
mod framing;
#[path = "header.rs"]
mod header;

use std::collections::BTreeSet;

pub use framing::Kind;
pub use header::{Token, inspect_header};

pub struct Roster {
    pub boundary: usize,
    pub declared: BTreeSet<String>,
    pub shapes: Vec<String>,
    pub tail: Vec<(Kind, Vec<Token>)>,
}

pub fn word(token: &Token, text: &str) -> bool {
    !token.quoted && token.text == text
}

pub fn name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
        && bytes.all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

pub fn inspect(bytes: &[u8]) -> Result<Roster, String> {
    let (kinds, boundary) = framing::frame(bytes)?;
    let text = std::str::from_utf8(bytes).map_err(|_| "non-UTF8 input")?;
    let statements = inspect_header(text)?;
    if kinds.len() != statements.len() {
        return Err("statement/line mismatch".into());
    }
    let mut report = Roster {
        boundary,
        declared: BTreeSet::new(),
        shapes: Vec::new(),
        tail: Vec::new(),
    };
    let mut transform: Option<String> = None;
    let mut shape: Option<String> = None;
    let mut next_array = 0;
    let mut tail = false;
    for (kind, tokens) in kinds.into_iter().zip(statements).skip(3) {
        if tail {
            report.tail.push((kind, tokens));
            continue;
        }
        match kind {
            Kind::Header => return Err("extra header".into()),
            Kind::Transform => {
                if transform.is_some() && next_array != 3 {
                    return Err("incomplete previous mesh".into());
                }
                let [command, node_type, flag, node] = tokens.as_slice() else {
                    return Err("transform declaration".into());
                };
                if !word(command, "createNode")
                    || !word(node_type, "transform")
                    || !word(flag, "-n")
                    || !node.quoted
                    || !name(&node.text)
                    || !report.declared.insert(node.text.clone())
                {
                    return Err("transform name".into());
                }
                transform = Some(node.text.clone());
                shape = None;
                next_array = 0;
            }
            Kind::Mesh => {
                let [command, node_type, flag, node, parent_flag, parent] = tokens.as_slice()
                else {
                    return Err("mesh declaration".into());
                };
                if transform.is_none()
                    || shape.is_some()
                    || !word(command, "createNode")
                    || !word(node_type, "mesh")
                    || !word(flag, "-n")
                    || !word(parent_flag, "-p")
                    || !node.quoted
                    || !parent.quoted
                    || !name(&node.text)
                    || Some(parent.text.as_str()) != transform.as_deref()
                    || !report.declared.insert(node.text.clone())
                {
                    return Err("mesh owner".into());
                }
                shape = Some(node.text.clone());
                report.shapes.push(node.text.clone());
            }
            Kind::Uuid => {
                if transform.is_none() || next_array != 0 {
                    return Err("UUID owner".into());
                }
            }
            Kind::Channel => {
                if transform.is_none() || shape.is_some() {
                    return Err("transform channel".into());
                }
            }
            Kind::Array => {
                if shape.is_none() || next_array >= 3 {
                    return Err("array owner".into());
                }
                next_array += 1;
            }
            Kind::Lambert | Kind::Connection => {
                tail = true;
                report.tail.push((kind, tokens));
            }
            Kind::Color | Kind::Group => return Err("tail before declaration".into()),
        }
    }
    if transform.is_some() && next_array != 3 {
        return Err("incomplete geometry".into());
    }
    Ok(report)
}
