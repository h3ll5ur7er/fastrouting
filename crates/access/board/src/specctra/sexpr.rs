use crate::BoardError;

/// One node of a Specctra file: a token or a parenthesised list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Sexp {
    Atom { text: String, line: usize },
    List { items: Vec<Sexp>, line: usize },
}

impl Sexp {
    pub(crate) fn line(&self) -> usize {
        match self {
            Self::Atom { line, .. } | Self::List { line, .. } => *line,
        }
    }

    pub(crate) fn atom(&self) -> Option<&str> {
        match self {
            Self::Atom { text, .. } => Some(text),
            Self::List { .. } => None,
        }
    }

    /// The first token of a list, which names what the list is.
    pub(crate) fn keyword(&self) -> Option<&str> {
        match self {
            Self::List { items, .. } => items.first().and_then(Self::atom),
            Self::Atom { .. } => None,
        }
    }

    /// Everything after the keyword.
    pub(crate) fn args(&self) -> &[Sexp] {
        match self {
            Self::List { items, .. } if !items.is_empty() => &items[1..],
            _ => &[],
        }
    }

    pub(crate) fn children<'s>(&'s self, keyword: &'s str) -> impl Iterator<Item = &'s Sexp> {
        self.args()
            .iter()
            .filter(move |item| item.keyword() == Some(keyword))
    }

    pub(crate) fn child(&self, keyword: &str) -> Option<&Sexp> {
        self.args()
            .iter()
            .find(|item| item.keyword() == Some(keyword))
    }

    pub(crate) fn atoms(&self) -> impl Iterator<Item = &str> {
        self.args().iter().filter_map(Self::atom)
    }

    pub(crate) fn invalid(&self, message: impl Into<String>) -> BoardError {
        BoardError::Invalid {
            line: self.line(),
            message: message.into(),
        }
    }

    pub(crate) fn required_child(&self, keyword: &str) -> Result<&Sexp, BoardError> {
        self.child(keyword)
            .ok_or_else(|| self.invalid(format!("missing ({keyword} …)")))
    }

    pub(crate) fn arg_atom(&self, index: usize, what: &str) -> Result<&str, BoardError> {
        self.args()
            .get(index)
            .and_then(Self::atom)
            .ok_or_else(|| self.invalid(format!("missing {what}")))
    }
}

/// Parses one top-level list. Specctra files may redefine their quote character
/// with `(string_quote X)`, where X is a bare character, so quoting is stateful.
pub(crate) fn parse(text: &str) -> Result<Sexp, BoardError> {
    let mut quote = '"';
    let mut line = 1;
    let mut open: Vec<(Vec<Sexp>, usize)> = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\n' => line += 1,
            c if c.is_whitespace() => {}
            '(' => open.push((Vec::new(), line)),
            ')' => {
                let (items, start) = open.pop().ok_or_else(|| BoardError::Syntax {
                    line,
                    message: "unexpected ')'".into(),
                })?;
                let list = Sexp::List { items, line: start };
                match open.last_mut() {
                    Some((parent, _)) => parent.push(list),
                    None => return Ok(list),
                }
            }
            c => {
                let (items, _) = open.last_mut().ok_or_else(|| BoardError::Syntax {
                    line,
                    message: "text outside of a list".into(),
                })?;
                let defines_quote = items.len() == 1 && items[0].atom() == Some("string_quote");
                let text = if defines_quote {
                    quote = c;
                    c.to_string()
                } else if c == quote {
                    let mut quoted = String::new();
                    loop {
                        match chars.next() {
                            Some(q) if q == quote => break,
                            Some(other) => {
                                line += usize::from(other == '\n');
                                quoted.push(other);
                            }
                            None => {
                                return Err(BoardError::Syntax {
                                    line,
                                    message: "unterminated string".into(),
                                });
                            }
                        }
                    }
                    quoted
                } else {
                    let mut token = c.to_string();
                    while let Some(&next) = chars.peek() {
                        if next.is_whitespace() || next == '(' || next == ')' {
                            break;
                        }
                        token.push(next);
                        chars.next();
                    }
                    token
                };
                items.push(Sexp::Atom { text, line });
            }
        }
    }
    match open.last() {
        Some((_, start)) => Err(BoardError::Syntax {
            line: *start,
            message: "list is never closed".into(),
        }),
        None => Err(BoardError::Syntax {
            line,
            message: "empty file".into(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atoms(sexp: &Sexp) -> Vec<&str> {
        sexp.atoms().collect()
    }

    #[test]
    fn parses_nested_lists() {
        let root = parse("(pcb board (unit um))").expect("valid");
        assert_eq!(root.keyword(), Some("pcb"));
        assert_eq!(atoms(root.child("unit").expect("unit")), ["um"]);
    }

    #[test]
    fn quoted_tokens_keep_spaces_and_parentheses() {
        let root = parse("(net \"a (b) c\" x)").expect("valid");
        assert_eq!(atoms(&root), ["a (b) c", "x"]);
    }

    #[test]
    fn string_quote_redefines_the_quote_character() {
        let root = parse("(pcb (parser (string_quote ')) (net 'a b'))").expect("valid");
        assert_eq!(atoms(root.child("net").expect("net")), ["a b"]);
    }

    #[test]
    fn the_default_quote_can_be_declared_as_a_bare_character() {
        let root = parse("(pcb (parser (string_quote \")) (net \"a b\"))").expect("valid");
        assert_eq!(atoms(root.child("net").expect("net")), ["a b"]);
    }

    #[test]
    fn counts_lines() {
        let root = parse("(pcb\n\n  (unit um))").expect("valid");
        assert_eq!(root.child("unit").expect("unit").line(), 3);
    }

    #[test]
    fn an_unclosed_list_is_reported_where_it_opens() {
        let error = parse("(pcb\n  (structure\n").unwrap_err();
        assert!(
            matches!(error, BoardError::Syntax { line: 2, .. }),
            "{error}"
        );
    }

    #[test]
    fn a_stray_closing_parenthesis_is_an_error() {
        assert!(parse(")").is_err());
    }
}
