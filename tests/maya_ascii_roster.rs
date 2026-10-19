#[path = "support/maya_ascii_structure/fixtures.rs"]
mod fixtures;
#[path = "support/maya_ascii_structure/roster.rs"]
mod roster;

use fixtures::{CONNECTIONS, GEOMETRY, HEADER, MATERIALS, golden};
use roster::{Kind, inspect, inspect_header};

fn two_shape_prefix() -> String {
    let other = GEOMETRY.replace("plain", "other");
    format!("{HEADER}{GEOMETRY}{other}")
}

fn refuses(document: &str, category: &str) {
    assert_eq!(
        inspect(document.as_bytes()).err().as_deref(),
        Some(category)
    );
}

#[test]
fn empty_one_and_two_shape_rosters_preserve_all_geometry_observations() {
    let empty = inspect(HEADER.as_bytes()).unwrap();
    assert_eq!(empty.boundary, HEADER.len());
    assert!(empty.declared.is_empty() && empty.shapes.is_empty() && empty.tail.is_empty());
    for (document, shapes, names) in [
        (
            format!("{HEADER}{GEOMETRY}"),
            vec!["plainShape"],
            vec!["plain", "plainShape"],
        ),
        (
            two_shape_prefix(),
            vec!["plainShape", "otherShape"],
            vec!["other", "otherShape", "plain", "plainShape"],
        ),
    ] {
        let report = inspect(document.as_bytes()).unwrap();
        assert_eq!(report.boundary, document.len());
        assert_eq!(report.shapes, shapes);
        assert_eq!(
            report
                .declared
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            names
        );
        assert!(report.tail.is_empty());
    }
}

#[test]
fn lambert_tail_keeps_exact_kinds_token_values_and_quote_flags() {
    let report = inspect(golden().as_bytes()).unwrap();
    assert_eq!(report.boundary, HEADER.len() + GEOMETRY.len());
    assert_eq!(report.shapes, ["plainShape"]);
    assert_eq!(
        report.declared.into_iter().collect::<Vec<_>>(),
        ["plain", "plainShape"]
    );
    let expected: Vec<_> = inspect_header(&format!("{HEADER}{MATERIALS}{CONNECTIONS}"))
        .unwrap()
        .into_iter()
        .skip(3)
        .collect();
    let (kinds, tokens): (Vec<_>, Vec<_>) = report.tail.into_iter().unzip();
    assert_eq!(tokens, expected);
    assert_eq!(
        kinds,
        [
            Kind::Lambert,
            Kind::Color,
            Kind::Group,
            Kind::Lambert,
            Kind::Color,
            Kind::Group,
            Kind::Connection,
            Kind::Connection,
            Kind::Connection,
        ]
    );
    let connection = format!("{HEADER}{GEOMETRY}{CONNECTIONS}");
    let report = inspect(connection.as_bytes()).unwrap();
    assert_eq!(report.boundary, HEADER.len() + GEOMETRY.len());
    assert_eq!(
        report
            .tail
            .iter()
            .map(|(kind, _)| *kind)
            .collect::<Vec<_>>(),
        [Kind::Connection; 3]
    );
    let expected: Vec<_> = inspect_header(&format!("{HEADER}{CONNECTIONS}"))
        .unwrap()
        .into_iter()
        .skip(3)
        .collect();
    assert_eq!(
        report
            .tail
            .into_iter()
            .map(|(_, tokens)| tokens)
            .collect::<Vec<_>>(),
        expected
    );
}

#[test]
fn optional_uuid_and_transform_channel_positions_have_checked_owners() {
    let uuid = "\trename -uid \"id\";\n";
    let channel = "\tsetAttr \".t\" -type \"double3\" 0 0 0;\n";
    let geometry = GEOMETRY
        .replace(
            "createNode mesh",
            &format!("{uuid}{channel}createNode mesh"),
        )
        .replace(
            "\tsetAttr -s 4 \".vt",
            &format!("{uuid}\tsetAttr -s 4 \".vt"),
        );
    let document = format!("{HEADER}{geometry}");
    let report = inspect(document.as_bytes()).unwrap();
    assert_eq!(report.boundary, document.len());
    assert_eq!(report.shapes, ["plainShape"]);
    let second = geometry.replace("plain", "other");
    let document = format!("{HEADER}{geometry}{second}");
    let report = inspect(document.as_bytes()).unwrap();
    assert_eq!(report.boundary, document.len());
    assert_eq!(report.shapes, ["plainShape", "otherShape"]);
    for (owner, anchor) in [
        ("transform", "createNode mesh"),
        ("mesh", "\tsetAttr -s 4 \".vt"),
    ] {
        for (case, statement) in [
            ("duplicate", format!("{uuid}{uuid}")),
            ("spaced missing", "\trename -uid ;\n".into()),
            ("bare", "\trename -uid bare;\n".into()),
            ("empty quoted", "\trename -uid \"\";\n".into()),
            ("extra token", "\trename -uid \"id\" extra;\n".into()),
        ] {
            let malformed = GEOMETRY.replace(anchor, &format!("{statement}{anchor}"));
            assert_eq!(
                inspect(format!("{HEADER}{malformed}").as_bytes())
                    .err()
                    .as_deref(),
                Some("UUID owner"),
                "{owner}: {case}"
            );
        }
    }
    refuses(&format!("{HEADER}{uuid}{GEOMETRY}"), "UUID owner");
    refuses(&format!("{HEADER}{GEOMETRY}{uuid}"), "UUID owner");
    refuses(&format!("{HEADER}{channel}{GEOMETRY}"), "transform channel");
    refuses(
        &format!(
            "{HEADER}{}",
            GEOMETRY.replace(
                "\tsetAttr -s 4 \".vt",
                &format!("{channel}\tsetAttr -s 4 \".vt")
            )
        ),
        "transform channel",
    );
}

#[test]
fn malformed_declarations_names_parents_and_duplicate_owners_fail() {
    let document = format!("{HEADER}{GEOMETRY}");
    for declaration in [
        "createNode transform -n plain;",
        "createNode transform -x \"plain\";",
    ] {
        refuses(
            &document.replace("createNode transform -n \"plain\";", declaration),
            "transform name",
        );
    }
    assert!(inspect(document.replace("\"plain\"", "\"valid_name\"").as_bytes()).is_ok());
    for name in ["1bad", ""] {
        refuses(
            &document.replace("\"plain\"", &format!("\"{name}\"")),
            "transform name",
        );
    }
    refuses(
        &document.replace("-n \"plain\";", "-n \"plain\" extra;"),
        "transform declaration",
    );
    for mesh in [
        "createNode mesh -n plainShape -p \"plain\";",
        "createNode mesh -n \"1bad\" -p \"plain\";",
        "createNode mesh -n \"plain\" -p \"plain\";",
        "createNode mesh -n \"plainShape\" -p \"other\";",
        "createNode mesh -n \"plainShape\" -p plain;",
    ] {
        refuses(
            &document.replace("createNode mesh -n \"plainShape\" -p \"plain\";", mesh),
            "mesh owner",
        );
    }
    refuses(
        &document.replace("createNode transform -n \"plain\";\n", ""),
        "mesh owner",
    );
    refuses(&document.replace("-p \"plain\";", ";"), "mesh declaration");
    refuses(
        &document.replace(
            "\tsetAttr -s 4 \".vt",
            "createNode mesh -n \"extra\" -p \"plain\";\n\tsetAttr -s 4 \".vt",
        ),
        "mesh owner",
    );
    refuses(
        &two_shape_prefix().replace("other", "plain"),
        "transform name",
    );
    refuses(
        &two_shape_prefix().replace("other\"", "plainShape\""),
        "transform name",
    );
}

#[test]
fn geometry_completion_and_array_ownership_are_independent_of_tail_semantics() {
    let face = "\tsetAttr -s 1 \".fc[0:0]\" -type \"polyFaces\"\n\tf 4 0 1 2 3\n;\n";
    let incomplete = format!("{HEADER}{}", GEOMETRY.strip_suffix(face).unwrap());
    refuses(&incomplete, "incomplete geometry");
    refuses(&format!("{incomplete}{MATERIALS}"), "incomplete geometry");
    refuses(
        &format!("{incomplete}{}", GEOMETRY.replace("plain", "other")),
        "incomplete previous mesh",
    );
    refuses(&format!("{HEADER}{face}{GEOMETRY}"), "array owner");
    refuses(&format!("{HEADER}{GEOMETRY}{face}"), "array owner");
    refuses(
        &format!("{HEADER}\tsetAttr \".c\" -type \"float3\" 0 0 0;\n"),
        "tail before declaration",
    );
    refuses(
        &format!("{HEADER}createNode shadingEngine -n \"group\";\n"),
        "tail before declaration",
    );
}

#[test]
fn whole_input_framing_and_tokenizer_refusals_propagate() {
    refuses(&golden().replace('\n', "\r\n"), "canonical LF framing");
    refuses(
        &format!("{}evalDeferred \"script\";\n", golden()),
        "unknown body statement",
    );
    refuses(
        &format!("{}createNode transform -n \"late\";\n", golden()),
        "geometry after material tail",
    );
    refuses(
        &golden().replace("\t-0.5 0 -0.5\n", "\t\n"),
        "array row framing",
    );
    refuses(
        &golden().replace(
            "createNode lambert -n \"red\";",
            "createNode lambert -n \"r\\q\";",
        ),
        "unsupported string escape",
    );
}
