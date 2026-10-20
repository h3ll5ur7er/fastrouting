use crate::BoardError;
use crate::sexp::Sexp;

/// Parses KiCad's S-expressions: quoted strings escape quotes and backslashes
/// with a backslash, and keywords are written in lower case.
pub(crate) fn parse(text: &str) -> Result<Sexp, BoardError> {
    let bytes = text.as_bytes();
    let mut line = 1;
    let mut open: Vec<(Vec<Sexp>, usize)> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                line += 1;
                i += 1;
            }
            b if b.is_ascii_whitespace() => i += 1,
            b'(' => {
                open.push((Vec::new(), line));
                i += 1;
            }
            b')' => {
                let (items, start) = open.pop().ok_or_else(|| BoardError::Syntax {
                    line,
                    message: "unexpected ')'".into(),
                })?;
                let list = Sexp::List { items, line: start };
                match open.last_mut() {
                    Some((parent, _)) => parent.push(list),
                    None => return Ok(list),
                }
                i += 1;
            }
            first => {
                let (items, _) = open.last_mut().ok_or_else(|| BoardError::Syntax {
                    line,
                    message: "text outside of a list".into(),
                })?;
                let at = line;
                let text = if first == b'"' {
                    let (quoted, end, lines) = quoted(text, i + 1).ok_or(BoardError::Syntax {
                        line,
                        message: "unterminated string".into(),
                    })?;
                    line += lines;
                    i = end;
                    quoted
                } else {
                    let start = i;
                    while i < bytes.len()
                        && !bytes[i].is_ascii_whitespace()
                        && !matches!(bytes[i], b'(' | b')')
                    {
                        i += 1;
                    }
                    text[start..i].to_owned()
                };
                items.push(Sexp::Atom { text, line: at });
            }
        }
    }
    Err(BoardError::Syntax {
        line: open.last().map_or(line, |(_, start)| *start),
        message: if open.is_empty() {
            "empty file".into()
        } else {
            "list is never closed".into()
        },
    })
}

/// The string that starts at byte `start`, after its opening quote: its text,
/// the byte after its closing quote, and how many lines it spans.
fn quoted(text: &str, start: usize) -> Option<(String, usize, usize)> {
    let mut string = String::new();
    let mut lines = 0;
    let mut chars = text[start..].char_indices();
    while let Some((offset, c)) = chars.next() {
        match c {
            '"' => return Some((string, start + offset + 1, lines)),
            '\\' => match chars.next()?.1 {
                'n' => string.push('\n'),
                escaped => string.push(escaped),
            },
            c => {
                lines += usize::from(c == '\n');
                string.push(c);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atoms(sexp: &Sexp) -> Vec<&str> {
        sexp.atoms().collect()
    }

    #[test]
    fn parses_nested_lists() {
        let root =
            parse("(kicad_pcb (version 20260206) (layers (0 \"F.Cu\" signal)))").expect("valid");
        assert_eq!(root.keyword(), Some("kicad_pcb"));
        assert_eq!(atoms(root.child("version").expect("version")), ["20260206"]);
        let layer = &root.child("layers").expect("layers").args()[0];
        assert_eq!(layer.keyword(), Some("0"));
        assert_eq!(atoms(layer), ["F.Cu", "signal"]);
    }

    #[test]
    fn quoted_strings_keep_spaces_and_parentheses() {
        let root = parse("(net \"a (b) c\")").expect("valid");
        assert_eq!(atoms(&root), ["a (b) c"]);
    }

    #[test]
    fn escaped_quotes_and_backslashes_are_part_of_a_string() {
        let root = parse(r#"(property "Value" "0.5\" \\ x")"#).expect("valid");
        assert_eq!(atoms(&root), ["Value", "0.5\" \\ x"]);
    }

    #[test]
    fn an_empty_string_is_a_token() {
        let root = parse("(net 0 \"\")").expect("valid");
        assert_eq!(atoms(&root), ["0", ""]);
    }

    #[test]
    fn counts_lines() {
        let root = parse("(kicad_pcb\n\t(net \"a\nb\")\n\t(version 1))").expect("valid");
        assert_eq!(root.child("version").expect("version").line(), 4);
    }

    #[test]
    fn an_unclosed_list_or_string_is_an_error() {
        assert!(matches!(
            parse("(kicad_pcb\n  (net \"a\")\n  (layers"),
            Err(BoardError::Syntax { line: 3, .. })
        ));
        assert!(parse("(net \"a)").is_err());
        assert!(parse(")").is_err());
    }
}
