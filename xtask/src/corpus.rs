//! Routes real boards that other people made and judges the result:
//! `cargo xtask corpus [--timeout SECONDS] [--freerouting CMD] [--shard K/N]
//! [--list FILE]`. Each board is routed by traxide and, when given, by
//! Freerouting; `traxide check` judges both sessions alike, against the rules
//! the board itself already breaks. The boards are downloaded at run time into
//! `target/corpus`, never committed. Prints a Markdown table, and appends it to
//! `$GITHUB_STEP_SUMMARY` in CI. traxide's results land in
//! `target/corpus/results-K-of-N.json`, for `cargo xtask gate` to compare
//! with the baseline.

use crate::gate::{Outcome, Record};
use crate::kicad;
use anyhow::{Context, Result, ensure};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const DEFAULT_LIST: &str = "corpus/freerouting-fixtures.txt";
const SOURCE: &str = "https://raw.githubusercontent.com/freerouting/freerouting/master/";

pub fn run(args: &[String]) -> Result<()> {
    let timeout = Duration::from_secs(option(args, "--timeout").map_or(Ok(120), |t| t.parse())?);
    let freerouting = option(args, "--freerouting");
    let shard = option(args, "--shard").map_or(Ok((1, 1)), |s| shard(&s))?;
    let (root, traxide) = kicad::build_traxide()?;
    let list = root.join(option(args, "--list").unwrap_or_else(|| DEFAULT_LIST.into()));
    let folder = root.join("target/corpus");
    std::fs::create_dir_all(folder.join("routed"))?;
    let text = std::fs::read_to_string(&list).with_context(|| list.display().to_string())?;
    let mut rows = Vec::new();
    for board in in_shard(boards(&text), shard) {
        let file = folder.join(board.replace(['/', ' '], "_"));
        if !file.exists() {
            download(board, &file)?;
        }
        let name = file
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let session = |router: &str| folder.join("routed").join(format!("{name}.{router}.ses"));
        let mut traxide_run = Command::new(&traxide);
        traxide_run
            .arg("route")
            .arg("--quiet")
            .arg(&file)
            .arg("-o")
            .arg(session("traxide"));
        let mut row = Row {
            board: name.clone(),
            before: judge(&traxide, &file, None).ok(),
            traxide: attempt(&traxide, &file, traxide_run, &session("traxide"), timeout)?,
            freerouting: None,
        };
        if let Some(command) = &freerouting {
            let mut words = command.split_whitespace();
            let mut run = Command::new(words.next().context("--freerouting names no command")?);
            run.args(words)
                .arg("-de")
                .arg(&file)
                .arg("-do")
                .arg(session("freerouting"))
                .arg("--gui.enabled=false");
            row.freerouting = Some(attempt(
                &traxide,
                &file,
                run,
                &session("freerouting"),
                timeout,
            )?);
        }
        eprintln!("{}", row.cells());
        rows.push(row);
    }
    let record = Record {
        timeout_seconds: timeout.as_secs(),
        boards: rows
            .iter()
            .map(|row| (row.board.clone(), row.outcome()))
            .collect(),
    };
    let results = folder.join(format!("results-{}-of-{}.json", shard.0, shard.1));
    std::fs::write(&results, serde_json::to_string_pretty(&record)? + "\n")?;
    eprintln!("wrote {}", results.display());
    let table = table(&rows, freerouting.is_some());
    println!("{table}");
    if let Ok(summary) = std::env::var("GITHUB_STEP_SUMMARY") {
        std::fs::write(summary, format!("## Corpus\n\n{table}\n"))?;
    }
    Ok(())
}

fn boards(list: &str) -> impl Iterator<Item = &str> {
    list.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

/// "K/N": the K-th of N shards, counting from 1.
fn shard(text: &str) -> Result<(usize, usize)> {
    let (k, n) = text.split_once('/').context("--shard takes K/N")?;
    let (k, n) = (k.parse()?, n.parse()?);
    ensure!(
        (1..=n).contains(&k),
        "--shard {text}: K must be between 1 and N"
    );
    Ok((k, n))
}

/// Every N-th board, starting at the K-th: shards of about equal size.
fn in_shard<'a>(
    boards: impl Iterator<Item = &'a str>,
    (k, n): (usize, usize),
) -> impl Iterator<Item = &'a str> {
    boards
        .enumerate()
        .filter(move |(i, _)| i % n == k - 1)
        .map(|(_, b)| b)
}

fn download(board: &str, file: &Path) -> Result<()> {
    let url = format!("{SOURCE}{}", board.replace(' ', "%20").replace('+', "%2B"));
    let status = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--output",
        ])
        .arg(file)
        .arg(&url)
        .status()
        .context("curl")?;
    ensure!(status.success(), "downloading {url} failed");
    Ok(())
}

struct Row {
    board: String,
    /// The board as it came, before either router.
    before: Option<Judgement>,
    traxide: Attempt,
    freerouting: Option<Attempt>,
}

/// What `traxide check --json` says about a board and its routes.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Judgement {
    nets: usize,
    incomplete_nets: usize,
    violations: usize,
}

#[derive(Debug)]
enum Attempt {
    Judged { seconds: f64, judgement: Judgement },
    Stopped { seconds: f64 },
    Failed { why: String },
}

/// Runs a router and judges its session.
fn attempt(
    traxide: &Path,
    board: &Path,
    mut router: Command,
    session: &Path,
    timeout: Duration,
) -> Result<Attempt> {
    // A session left from an earlier run must not pass for this one's.
    let _ = std::fs::remove_file(session);
    let started = Instant::now();
    let mut child = router
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    let finished = loop {
        if child.try_wait()?.is_some() {
            break true;
        }
        if started.elapsed() > timeout {
            child.kill()?;
            break false;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let seconds = started.elapsed().as_secs_f64();
    let stderr = child.wait_with_output()?.stderr;
    Ok(if !finished {
        Attempt::Stopped { seconds }
    } else if !session.exists() {
        Attempt::Failed {
            why: first_line(&String::from_utf8_lossy(&stderr)),
        }
    } else {
        match judge(traxide, board, Some(session)) {
            Ok(judgement) => Attempt::Judged { seconds, judgement },
            Err(why) => Attempt::Failed {
                why: quoting_line(&why, &std::fs::read_to_string(session).unwrap_or_default()),
            },
        }
    })
}

fn judge(traxide: &Path, board: &Path, session: Option<&Path>) -> Result<Judgement, String> {
    let mut check = Command::new(traxide);
    check.arg("check").arg(board).args(session).arg("--json");
    let output = check.output().map_err(|e| e.to_string())?;
    judgement(&String::from_utf8_lossy(&output.stdout))
        .ok_or_else(|| first_line(&String::from_utf8_lossy(&output.stderr)))
}

/// Reads the report of `traxide check --json`.
fn judgement(json: &str) -> Option<Judgement> {
    let report: serde_json::Value = serde_json::from_str(json).ok()?;
    let count = |key: &str| report[key].as_u64().and_then(|n| usize::try_from(n).ok());
    Some(Judgement {
        nets: count("nets")?,
        incomplete_nets: count("incomplete_nets")?,
        violations: report["violations"]
            .as_object()?
            .values()
            .filter_map(serde_json::Value::as_u64)
            .map(|n| usize::try_from(n).unwrap_or(usize::MAX))
            .sum(),
    })
}

/// `why`, a reason that names a line of the session, with that line, so that
/// the table shows what the router wrote there.
fn quoting_line(why: &str, session: &str) -> String {
    let number = why
        .strip_prefix("line ")
        .and_then(|rest| rest.split_once(':'))
        .and_then(|(number, _)| number.parse::<usize>().ok());
    match number.and_then(|n| session.lines().nth(n.checked_sub(1)?)) {
        Some(line) => {
            let line: String = line.trim().chars().take(80).collect();
            format!("{why}, at `{}`", line.replace('|', "/"))
        }
        None => why.to_owned(),
    }
}

fn first_line(text: &str) -> String {
    text.lines()
        .next()
        .unwrap_or("")
        .trim_start_matches("error: ")
        .replace('|', "/")
}

impl Row {
    /// traxide's attempt, as the gate compares it.
    fn outcome(&self) -> Outcome {
        let tenths = |seconds: f64| (seconds * 10.0).round() / 10.0;
        match &self.traxide {
            Attempt::Judged { seconds, judgement } => Outcome::Finished {
                seconds: tenths(*seconds),
                nets: judgement.nets,
                routed: judgement.nets - judgement.incomplete_nets,
                rules_broken: judgement
                    .violations
                    .saturating_sub(self.before.map_or(0, |b| b.violations)),
            },
            Attempt::Stopped { seconds } => Outcome::Stopped {
                seconds: tenths(*seconds),
            },
            Attempt::Failed { why } => Outcome::Failed { why: why.clone() },
        }
    }

    fn cells(&self) -> String {
        let mut cells = vec![self.board.clone(), self.describe(&self.traxide)];
        if let Some(freerouting) = &self.freerouting {
            cells.push(self.describe(freerouting));
        }
        format!("| {} |", cells.join(" | "))
    }

    /// Time, complete nets, and the rules broken beyond those the board broke.
    fn describe(&self, attempt: &Attempt) -> String {
        let broken_before = self.before.map_or(0, |b| b.violations);
        match attempt {
            Attempt::Judged { seconds, judgement } => format!(
                "{seconds:.1} s, {} of {} nets, {} rules broken",
                judgement.nets - judgement.incomplete_nets,
                judgement.nets,
                judgement.violations.saturating_sub(broken_before)
            ),
            Attempt::Stopped { seconds } => format!("stopped after {seconds:.0} s"),
            Attempt::Failed { why } => format!("failed: {why}"),
        }
    }
}

fn table(rows: &[Row], with_freerouting: bool) -> String {
    let mut lines = vec![
        if with_freerouting {
            "| Board | traxide | Freerouting |\n| --- | --- | --- |"
        } else {
            "| Board | traxide |\n| --- | --- |"
        }
        .to_owned(),
    ];
    lines.extend(rows.iter().map(Row::cells));
    lines.push(String::new());
    lines.push(totals("traxide", rows.iter().map(|r| (r, &r.traxide))));
    if with_freerouting {
        lines.push(totals(
            "Freerouting",
            rows.iter()
                .filter_map(|r| r.freerouting.as_ref().map(|f| (r, f))),
        ));
    }
    lines.join("\n")
}

fn totals<'a>(router: &str, attempts: impl Iterator<Item = (&'a Row, &'a Attempt)>) -> String {
    let (mut complete, mut incomplete, mut broke, mut stopped, mut failed, mut seconds) =
        (0, 0, 0, 0, 0, 0.0);
    for (row, attempt) in attempts {
        match attempt {
            Attempt::Judged {
                seconds: s,
                judgement,
            } => {
                seconds += s;
                if judgement.violations > row.before.map_or(0, |b| b.violations) {
                    broke += 1;
                } else if judgement.incomplete_nets == 0 {
                    complete += 1;
                } else {
                    incomplete += 1;
                }
            }
            Attempt::Stopped { seconds: s } => {
                seconds += s;
                stopped += 1;
            }
            Attempt::Failed { .. } => failed += 1,
        }
    }
    format!(
        "{router}: {complete} complete, {incomplete} incomplete, {broke} breaking rules, {stopped} stopped, {failed} failed; {seconds:.0} s in all."
    )
}

fn option(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_report_of_traxide_check() {
        let json = r#"{"nets":31,"incomplete_nets":2,"unconnected_pads":3,
            "violations":{"clearance":4,"edge":1},"tracks":9,"vias":1,"track_length_mm":3.5}"#;
        assert_eq!(
            judgement(json),
            Some(Judgement {
                nets: 31,
                incomplete_nets: 2,
                violations: 5
            })
        );
    }

    #[test]
    fn an_error_at_a_line_quotes_that_line() {
        let session = "(session s\n  (routes\n    (via Round1 1 2)\n";
        assert_eq!(
            quoting_line("line 3: unknown via padstack Round1", session),
            "line 3: unknown via padstack Round1, at `(via Round1 1 2)`"
        );
        assert_eq!(quoting_line("no line", session), "no line");
    }

    #[test]
    fn an_error_is_no_report() {
        assert_eq!(judgement(""), None);
    }

    #[test]
    fn shards_take_every_nth_board() {
        let boards = ["a", "b", "c", "d", "e"];
        let second: Vec<&str> = in_shard(boards.into_iter(), (2, 2)).collect();
        assert_eq!(second, ["b", "d"]);
        assert!(shard("3/2").is_err());
    }

    #[test]
    fn rules_the_board_broke_already_are_not_counted_against_a_router() {
        let row = Row {
            board: "b".into(),
            before: Some(Judgement {
                nets: 4,
                incomplete_nets: 4,
                violations: 2,
            }),
            traxide: Attempt::Judged {
                seconds: 1.0,
                judgement: Judgement {
                    nets: 4,
                    incomplete_nets: 0,
                    violations: 2,
                },
            },
            freerouting: None,
        };
        assert_eq!(row.cells(), "| b | 1.0 s, 4 of 4 nets, 0 rules broken |");
        assert_eq!(
            row.outcome(),
            Outcome::Finished {
                seconds: 1.0,
                nets: 4,
                routed: 4,
                rules_broken: 0
            }
        );
    }
}
