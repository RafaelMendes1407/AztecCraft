// Independent inert oracle, with no production parser or emitter dependency.
#[derive(Debug, PartialEq, Eq)]
pub struct Token {
    pub text: String,
    pub quoted: bool,
}

fn statements(text: &str) -> Result<Vec<Vec<Token>>, String> {
    let (mut all, mut current, mut word) = (Vec::new(), Vec::new(), String::new());
    let mut chars = text.chars().peekable();
    let mut after_quote = false;
    while let Some(c) = chars.next() {
        let comment = c == '/' && chars.peek() == Some(&'/');
        if c == '"' && (!word.is_empty() || after_quote) {
            return Err("missing token separator".into());
        }
        if comment || c == ';' || c.is_ascii_whitespace() {
            if !word.is_empty() {
                current.push(Token {
                    text: std::mem::take(&mut word),
                    quoted: false,
                });
            }
            after_quote = false;
        }
        if c == '"' {
            let mut value = String::new();
            loop {
                match chars.next().ok_or("unterminated string")? {
                    '"' => break,
                    '\\' => match chars.next().ok_or("unterminated escape")? {
                        escaped @ ('"' | '\\') => value.push(escaped),
                        _ => return Err("unsupported string escape".into()),
                    },
                    c => value.push(c),
                }
            }
            current.push(Token {
                text: value,
                quoted: true,
            });
            after_quote = true;
        } else if comment {
            chars.by_ref().find(|c| *c == '\n');
        } else if c == ';' {
            all.push(std::mem::take(&mut current));
        } else if !c.is_ascii_whitespace() {
            if after_quote {
                return Err("missing token separator".into());
            }
            word.push(c);
        }
    }
    if current.is_empty() && word.is_empty() {
        Ok(all)
    } else {
        Err("statement not terminated by ';'".into())
    }
}

/// Success inspects framing and the prefix only; body tokens remain unvalidated data.
pub fn inspect_header(text: &str) -> Result<Vec<Vec<Token>>, String> {
    let all = statements(text)?;
    let expected: [&[(&str, bool)]; 3] = [
        &[("requires", false), ("maya", false), ("2026", true)],
        &[
            ("currentUnit", false),
            ("-l", false),
            ("centimeter", false),
            ("-a", false),
            ("degree", false),
            ("-t", false),
            ("film", false),
        ],
        &[("fileInfo", false), ("AztecCraft", true), ("1", true)],
    ];
    let prefix = all.get(..3).ok_or("incomplete canonical header")?;
    if !prefix.iter().zip(expected).all(|(actual, expected)| {
        actual
            .iter()
            .map(|t| (t.text.as_str(), t.quoted))
            .eq(expected.iter().copied())
    }) {
        return Err("incorrect canonical header".into());
    }
    if all.iter().skip(3).any(|statement| {
        matches!(statement.as_slice(), [command, key, ..]
            if !command.quoted && command.text == "fileInfo"
                && key.quoted && key.text == "AztecCraft")
    }) {
        return Err("duplicate AztecCraft marker".into());
    }
    Ok(all)
}
