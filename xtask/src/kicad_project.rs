//! KiCad projects in the corpus: a board in a GitHub repository at a fixed
//! commit, with the project and custom rules of the same name beside it, and
//! the board stripped of the copper its author's router laid.

use std::ops::Range;

/// One line of a corpus list: `https://github.com/OWNER/REPO COMMIT PATH.kicad_pcb`.
#[derive(Debug, PartialEq)]
pub struct Project<'a> {
    repository: &'a str,
    pub commit: &'a str,
    board: &'a str,
}

const GITHUB: &str = "https://github.com/";
const RAW_GITHUB: &str = "https://raw.githubusercontent.com/";

/// The files of a KiCad project that traxide reads, besides the board.
pub const RULE_FILES: [&str; 2] = ["kicad_pro", "kicad_dru"];

impl<'a> Project<'a> {
    /// The project a corpus line names, or `None` for a line that names
    /// something else, such as a fixture of Freerouting's.
    pub fn parse(line: &'a str) -> Option<Self> {
        let mut words = line.split_whitespace();
        let repository = words.next()?.strip_prefix(GITHUB)?.trim_end_matches('/');
        let (commit, board) = (words.next()?, words.next()?);
        (words.next().is_none() && board.ends_with(".kicad_pcb")).then_some(Self {
            repository,
            commit,
            board,
        })
    }

    /// The repository's name and the board's: `tokenboard-kb`.
    pub fn name(&self) -> String {
        let repository = self.repository.rsplit('/').next().unwrap_or_default();
        format!("{repository}-{}", self.stem())
    }

    /// The board's file name without its extension: `kb`.
    pub fn stem(&self) -> &'a str {
        let file = self.board.rsplit('/').next().unwrap_or_default();
        file.trim_end_matches(".kicad_pcb")
    }

    /// Where the board's file with `extension` lies at the pinned commit.
    pub fn url(&self, extension: &str) -> String {
        let path = self.board.trim_end_matches("kicad_pcb");
        format!(
            "{RAW_GITHUB}{}/{}/{path}{extension}",
            self.repository, self.commit
        )
    }
}

/// The board without its unlocked tracks, arcs and vias: what is left for a
/// router once the author's router is taken out of a routed board. Zones and
/// the copper the author locked stay; the rest of the text is unchanged.
pub fn strip_unlocked_copper(board: &str) -> String {
    let mut kept = String::with_capacity(board.len());
    let mut from = 0;
    for item in elements(board) {
        if is_unlocked_copper(&board[item.clone()]) {
            let indented = board[..item.start].trim_end_matches([' ', '\t']).len();
            kept.push_str(&board[from..indented.max(from)]);
            let rest = &board[item.end..];
            let line_end = ["\r\n", "\n"]
                .into_iter()
                .find(|end| rest.starts_with(end))
                .map_or(0, str::len);
            from = item.end + line_end;
        }
    }
    kept.push_str(&board[from..]);
    kept
}

fn is_unlocked_copper(item: &str) -> bool {
    let parts = elements(item);
    let keyword = parts.first().map(|k| &item[k.clone()]);
    matches!(keyword, Some("segment" | "arc" | "via"))
        && !parts.iter().any(|part| is_locked(&item[part.clone()]))
}

/// `locked`, as KiCad wrote it before version 7, or `(locked yes)`.
fn is_locked(part: &str) -> bool {
    if part == "locked" {
        return true;
    }
    let words: Vec<&str> = elements(part).into_iter().map(|w| &part[w]).collect();
    matches!(words.as_slice(), ["locked"] | ["locked", "yes"])
}

/// The spans of the elements directly inside the list that `text` starts
/// with: atoms, quoted strings and lists.
fn elements(text: &str) -> Vec<Range<usize>> {
    let bytes = text.as_bytes();
    let mut spans = Vec::new();
    let (mut depth, mut list_start, mut atom_start) = (0usize, 0, None);
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        let ends_atom = byte == b'(' || byte == b')' || byte == b'"' || byte.is_ascii_whitespace();
        if ends_atom && let Some(start) = atom_start.take() {
            spans.push(start..i);
        }
        match byte {
            b'"' => {
                let end = closing_quote(bytes, i);
                if depth == 1 {
                    spans.push(i..end + 1);
                }
                i = end;
            }
            b'(' => {
                depth += 1;
                if depth == 2 {
                    list_start = i;
                }
            }
            b')' => {
                if depth == 2 {
                    spans.push(list_start..i + 1);
                }
                if depth <= 1 {
                    break;
                }
                depth -= 1;
            }
            _ if depth == 1 && !ends_atom && atom_start.is_none() => atom_start = Some(i),
            _ => {}
        }
        i += 1;
    }
    spans
}

/// The index of the quote that closes the string opened at `open`.
fn closing_quote(bytes: &[u8], open: usize) -> usize {
    let mut i = open + 1;
    while i < bytes.len() && bytes[i] != b'"' {
        i += if bytes[i] == b'\\' { 2 } else { 1 };
    }
    i.min(bytes.len().saturating_sub(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: &str = "https://github.com/h3ll5ur7er/tokenboard 478b4f9 kb/kb.kicad_pcb";

    #[test]
    fn a_line_names_a_board_in_a_repository_at_a_commit() {
        let project = Project::parse(LINE).unwrap();
        assert_eq!(project.name(), "tokenboard-kb");
        assert_eq!(project.commit, "478b4f9");
        assert_eq!(
            project.url("kicad_dru"),
            "https://raw.githubusercontent.com/h3ll5ur7er/tokenboard/478b4f9/kb/kb.kicad_dru"
        );
    }

    #[test]
    fn a_fixture_of_freeroutings_is_no_project() {
        assert_eq!(Project::parse("fixtures/Issue015-StackOverflow.dsn"), None);
        assert_eq!(
            Project::parse("https://github.com/o/r 478b4f9 board.dsn"),
            None
        );
    }

    const ROUTED: &str = "(kicad_pcb\n\t(version 20260206)\n\
        \t(footprint \"SW\" (property \"Value\" \"(segment)\" (unlocked yes)) (pad \"1\" smd))\n\
        \t(segment\n\t\t(start 1 2)\n\t\t(end 3 4)\n\t\t(net \"COL1\")\n\t)\n\
        \t(segment (start 1 2) (end 3 4) (locked yes) (net \"COL1\"))\n\
        \t(arc (start 1 2) (mid 2 3) (end 3 4) (net \"locked\"))\n\
        \t(via locked (at 1 2) (net \"ROW1\"))\n\
        \t(via (at 5 6) (net \"ROW1\"))\n\
        \t(zone (net \"GND\") (polygon (pts (xy 0 0) (xy 9 0) (xy 9 9))))\n)\n";

    #[test]
    fn stripping_removes_unlocked_tracks_arcs_and_vias() {
        let stripped = strip_unlocked_copper(ROUTED);
        assert!(!stripped.contains("(end 3 4)\n"), "{stripped}");
        assert!(!stripped.contains("(arc"), "{stripped}");
        assert!(!stripped.contains("(at 5 6)"), "{stripped}");
    }

    #[test]
    fn stripping_keeps_locked_copper_zones_and_footprints_as_they_were() {
        assert_eq!(
            strip_unlocked_copper(ROUTED),
            "(kicad_pcb\n\t(version 20260206)\n\
            \t(footprint \"SW\" (property \"Value\" \"(segment)\" (unlocked yes)) (pad \"1\" smd))\n\
            \t(segment (start 1 2) (end 3 4) (locked yes) (net \"COL1\"))\n\
            \t(via locked (at 1 2) (net \"ROW1\"))\n\
            \t(zone (net \"GND\") (polygon (pts (xy 0 0) (xy 9 0) (xy 9 9))))\n)\n"
        );
    }

    #[test]
    fn a_board_without_copper_to_strip_stays_as_it_was() {
        let board = "(kicad_pcb (version 1) (net \"a\\\"(\"))";
        assert_eq!(strip_unlocked_copper(board), board);
    }
}
