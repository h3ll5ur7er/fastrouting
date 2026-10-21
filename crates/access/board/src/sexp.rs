use crate::BoardError;

/// One node of an S-expression file, as Specctra and KiCad write them: a token
/// or a parenthesised list.
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
