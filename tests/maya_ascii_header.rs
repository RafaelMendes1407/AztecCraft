#[path = "support/maya_ascii_structure/header.rs"]
mod header;

use header::{Token, inspect_header};

const HEADER: &str = concat!(
    "requires maya \"2026\";\n",
    "currentUnit -l centimeter -a degree -t film;\n",
    "fileInfo \"AztecCraft\" \"1\";\n"
);

fn pairs(tokens: &[Token]) -> Vec<(&str, bool)> {
    tokens.iter().map(|t| (t.text.as_str(), t.quoted)).collect()
}

fn rejects(text: &str, category: &str) {
    inspect_header(HEADER).unwrap();
    assert!(
        inspect_header(text).unwrap_err().contains(category),
        "expected {category}: {text:?}"
    );
}

#[test]
fn canonical_prefix_preserves_exact_text_and_quote_kinds() {
    let statements = inspect_header(HEADER).unwrap();
    assert_eq!(statements.len(), 3);
    assert_eq!(
        pairs(&statements[0]),
        vec![("requires", false), ("maya", false), ("2026", true)]
    );
    assert_eq!(
        pairs(&statements[1]),
        vec![
            ("currentUnit", false),
            ("-l", false),
            ("centimeter", false),
            ("-a", false),
            ("degree", false),
            ("-t", false),
            ("film", false),
        ]
    );
    assert_eq!(
        pairs(&statements[2]),
        vec![("fileInfo", false), ("AztecCraft", true), ("1", true)]
    );
}

#[test]
fn body_statements_are_inspected_inert_data_not_validated_scene_operations() {
    let text = format!("{HEADER}createNode transform -n \"sample\"; evalDeferred \"inert\";");
    let statements = inspect_header(&text).unwrap();
    assert_eq!(statements.len(), 5);
    assert_eq!(
        pairs(&statements[3]),
        vec![
            ("createNode", false),
            ("transform", false),
            ("-n", false),
            ("sample", true)
        ]
    );
    assert_eq!(
        pairs(&statements[4]),
        vec![("evalDeferred", false), ("inert", true)]
    );
}

#[test]
fn comments_and_ascii_whitespace_preserve_the_canonical_prefix() {
    let baseline = inspect_header(HEADER).unwrap();
    for text in [
        format!("// fileInfo \"AztecCraft\" \"1\"; evalDeferred\n{HEADER}// tail"),
        format!(" \t\r\n{}\r\n", HEADER.replace('\n', "\r\n")),
        HEADER.replace("maya ", "maya// boundary\n"),
        HEADER.replace("\"AztecCraft\" ", "\"AztecCraft\"// boundary\n"),
    ] {
        assert_eq!(inspect_header(&text).unwrap(), baseline);
    }
}

#[test]
fn missing_reordered_short_and_extended_prefix_statements_fail() {
    let lines: Vec<_> = HEADER.lines().collect();
    for line in &lines {
        rejects(&HEADER.replace(*line, ""), "header");
        let short = line.rsplit_once(' ').unwrap().0;
        rejects(&HEADER.replace(*line, &format!("{short};")), "header");
        rejects(
            &HEADER.replace(*line, &line.replace(';', " extra;")),
            "header",
        );
    }
    rejects(
        &format!("{}\n{}\n{}", lines[1], lines[0], lines[2]),
        "header",
    );
    rejects(
        &format!("{}\n{}\n{}", lines[0], lines[2], lines[1]),
        "header",
    );
}

#[test]
fn altered_prefix_values_and_token_kinds_fail() {
    for (from, to) in [
        ("requires", "\"requires\""),
        ("maya", "\"maya\""),
        ("maya", "mel"),
        ("\"2026\"", "2026"),
        ("2026", "2027"),
        ("\"2026\"", "evalDeferred"),
        ("currentUnit", "\"currentUnit\""),
        ("-l", "\"-l\""),
        ("centimeter", "\"centimeter\""),
        ("centimeter", "cm"),
        ("degree", "radian"),
        ("film", "pal"),
        ("fileInfo", "\"fileInfo\""),
        ("\"AztecCraft\"", "AztecCraft"),
        ("AztecCraft", "Other"),
        ("\"1\"", "1"),
        ("\"1\"", "\"2\""),
    ] {
        rejects(&HEADER.replacen(from, to, 1), "header");
    }
}

#[test]
fn tokens_require_whitespace_or_comment_separators() {
    for (from, to) in [
        ("maya \"2026\"", "maya\"2026\""),
        ("\"AztecCraft\" \"1\"", "\"AztecCraft\"\"1\""),
        ("\"2026\";", "\"2026\"extra;"),
    ] {
        rejects(&HEADER.replace(from, to), "separator");
    }
    for separator in [" ", "\t", "// boundary\n"] {
        inspect_header(&HEADER.replace("maya ", &format!("maya{separator}"))).unwrap();
    }
    for (joined, separated) in [
        (r#"data bare"quoted";"#, r#"data bare "quoted";"#),
        (r#"data "first""second";"#, r#"data "first" "second";"#),
        (r#"data "quoted"bare;"#, r#"data "quoted" bare;"#),
    ] {
        inspect_header(&format!("{HEADER}{separated}")).unwrap();
        rejects(&format!("{HEADER}{joined}"), "separator");
    }
}

#[test]
fn strings_preserve_escaped_quotes_backslashes_semicolons_and_comment_text() {
    let text = format!("{HEADER}data \"quote\\\" and slash\\\\; // inert\";");
    let statements = inspect_header(&text).unwrap();
    assert_eq!(statements.len(), 4);
    assert_eq!(
        pairs(&statements[3]),
        vec![("data", false), ("quote\" and slash\\; // inert", true)]
    );
}

#[test]
fn unknown_escapes_cannot_normalize_into_a_canonical_header() {
    for (from, to) in [
        ("\"2026\"", "\"\\2026\""),
        ("\"AztecCraft\"", "\"\\AztecCraft\""),
    ] {
        rejects(&HEADER.replace(from, to), "escape");
    }
    for escape in ["\\n", "\\q", "\\0", "\\\n"] {
        rejects(&format!("{HEADER}data \"{escape}\";"), "escape");
    }
}

#[test]
fn comments_and_quoted_payloads_cannot_replace_the_actual_marker() {
    let prefix = "requires maya \"2026\"; currentUnit -l centimeter -a degree -t film;";
    rejects(
        &format!("{prefix}// fileInfo \"AztecCraft\" \"1\";\n"),
        "header",
    );
    rejects(
        &format!("{prefix}data \"fileInfo \\\"AztecCraft\\\" \\\"1\\\";\";"),
        "header",
    );
}

#[test]
fn duplicate_actual_markers_fail_regardless_of_later_arity_or_version() {
    for suffix in [
        "fileInfo \"AztecCraft\" \"1\";",
        "fileInfo \"AztecCraft\" \"2\";",
        "fileInfo \"AztecCraft\";",
        "fileInfo \"AztecCraft\" \"1\" extra;",
    ] {
        rejects(&format!("{HEADER}{suffix}"), "duplicate");
    }
    for suffix in [
        "// fileInfo \"AztecCraft\" \"2\";",
        "data \"fileInfo \\\"AztecCraft\\\" \\\"2\\\";\";",
        "\"fileInfo\" \"AztecCraft\" \"2\";",
    ] {
        inspect_header(&format!("{HEADER}{suffix}")).unwrap();
    }
}

#[test]
fn incomplete_or_unterminated_input_returns_errors() {
    for text in [
        "",
        "requires maya \"2026\";",
        "requires maya \"2026\"; currentUnit -l centimeter;",
    ] {
        rejects(text, "header");
    }
    rejects(HEADER.trim_end().trim_end_matches(';'), "terminated");
    rejects(&format!("{HEADER}data \"unterminated"), "unterminated");
    rejects(&format!("{HEADER}data \"escape\\"), "unterminated");
    rejects(&format!("{HEADER}data bare"), "terminated");
}
