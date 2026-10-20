//! Routes real boards that other people made and reports how traxide fares:
//! `cargo xtask corpus [--timeout SECONDS] [--list FILE]`. The boards are
//! downloaded at run time into `target/corpus`, never committed. Prints a
//! Markdown table, and appends it to `$GITHUB_STEP_SUMMARY` in CI.

use crate::kicad;
use anyhow::{Context, Result, ensure};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const DEFAULT_LIST: &str = "corpus/freerouting-fixtures.txt";
const SOURCE: &str = "https://raw.githubusercontent.com/freerouting/freerouting/master/";

pub fn run(args: &[String]) -> Result<()> {
    let timeout = Duration::from_secs(option(args, "--timeout").map_or(Ok(120), |t| t.parse())?);
    let (root, traxide) = kicad::build_traxide()?;
    let list = root.join(option(args, "--list").unwrap_or_else(|| DEFAULT_LIST.into()));
    let folder = root.join("target/corpus");
    std::fs::create_dir_all(folder.join("routed"))?;
    let mut rows = Vec::new();
    for board in std::fs::read_to_string(&list)
        .with_context(|| list.display().to_string())?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let file = folder.join(board.replace(['/', ' '], "_"));
        if !file.exists() {
            download(board, &file)?;
        }
        rows.push(route(&traxide, &file, &folder.join("routed"), timeout)?);
    }
    let table = table(&rows);
    println!("{table}");
    if let Ok(summary) = std::env::var("GITHUB_STEP_SUMMARY") {
        std::fs::write(summary, format!("## Corpus\n\n{table}\n"))?;
    }
    Ok(())
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
    seconds: f64,
    outcome: Outcome,
}

#[derive(Debug, PartialEq)]
enum Outcome {
    Routed(Summary),
    Unreadable(String),
    Stopped,
}

/// What `traxide route` printed about a board it routed.
#[derive(Debug, Default, PartialEq)]
struct Summary {
    routed: usize,
    connections: usize,
    vias: usize,
    broken_rules: usize,
    broken_before_routing: usize,
}

fn route(traxide: &Path, board: &Path, out: &Path, timeout: Duration) -> Result<Row> {
    let name = board
        .file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let started = Instant::now();
    let mut child = Command::new(traxide)
        .arg("route")
        .arg("--quiet")
        .arg(board)
        .arg("-o")
        .arg(out.join(format!("{name}.ses")))
        .stdout(Stdio::piped())
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
    let output = child.wait_with_output()?;
    let outcome = if !finished {
        Outcome::Stopped
    } else {
        let stdout = String::from_utf8_lossy(&output.stdout);
        match summary(&stdout) {
            Some(summary) => Outcome::Routed(summary),
            None => Outcome::Unreadable(
                String::from_utf8_lossy(&output.stderr)
                    .trim()
                    .trim_start_matches("error: ")
                    .to_owned(),
            ),
        }
    };
    Ok(Row {
        board: name,
        seconds: started.elapsed().as_secs_f64(),
        outcome,
    })
}

/// Reads the summary `traxide route` prints: "Routed 9 of 10 connections (12
/// tracks, 3 vias) in …", and the lines about rules after it.
fn summary(stdout: &str) -> Option<Summary> {
    let first = stdout.lines().find(|l| l.starts_with("Routed "))?;
    let words: Vec<&str> = first.split_whitespace().collect();
    let number = |word: &str| word.trim_start_matches('(').parse::<usize>().ok();
    let count_in = |prefix: &str| {
        stdout
            .lines()
            .find_map(|l| l.strip_prefix(prefix))
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(number)
            .unwrap_or(0)
    };
    Some(Summary {
        routed: number(words.get(1)?)?,
        connections: number(words.get(3)?)?,
        vias: number(words.get(7)?)?,
        broken_rules: count_in("The routes break "),
        broken_before_routing: count_in("Note: the board broke "),
    })
}

fn table(rows: &[Row]) -> String {
    let mut lines = vec![
        "| Board | Time | Routed | Vias | Rules broken | Broken before routing |".to_owned(),
        "| --- | ---: | ---: | ---: | ---: | ---: |".to_owned(),
    ];
    let (mut complete, mut incomplete, mut broke, mut stopped, mut unreadable) = (0, 0, 0, 0, 0);
    for row in rows {
        let time = format!("{:.1} s", row.seconds);
        lines.push(match &row.outcome {
            Outcome::Routed(s) => {
                if s.broken_rules > 0 {
                    broke += 1;
                } else if s.routed == s.connections {
                    complete += 1;
                } else {
                    incomplete += 1;
                }
                format!(
                    "| {} | {time} | {} of {} | {} | {} | {} |",
                    row.board,
                    s.routed,
                    s.connections,
                    s.vias,
                    s.broken_rules,
                    s.broken_before_routing
                )
            }
            Outcome::Unreadable(why) => {
                unreadable += 1;
                format!("| {} | {time} | unreadable: {why} | | | |", row.board)
            }
            Outcome::Stopped => {
                stopped += 1;
                format!("| {} | > {time} (stopped) | | | | |", row.board)
            }
        });
    }
    lines.push(String::new());
    lines.push(format!(
        "{} boards: {complete} complete, {incomplete} incomplete, {broke} with rules traxide broke, {stopped} stopped, {unreadable} unreadable.",
        rows.len()
    ));
    lines.join("\n")
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
    fn reads_a_complete_summary() {
        let stdout = "Routed 10 of 10 connections (12 tracks, 3 vias) in 0.50 s → b.ses\n";
        assert_eq!(
            summary(stdout),
            Some(Summary {
                routed: 10,
                connections: 10,
                vias: 3,
                ..Summary::default()
            })
        );
    }

    #[test]
    fn reads_unrouted_connections_and_broken_rules() {
        let stdout = "Routed 8 of 10 connections (12 tracks, 3 vias) in 0.50 s → b.ses\n  \
                      unrouted: A from P1 to P2\n\
                      Note: the board broke 4 rules before routing, in the copper it came with; they are as they were.\n\
                      The routes break 2 rules. This is a bug in traxide; please report it with the board:\n";
        let read = summary(stdout).expect("a summary");
        assert_eq!(
            (read.routed, read.broken_rules, read.broken_before_routing),
            (8, 2, 4)
        );
    }

    #[test]
    fn an_error_is_no_summary() {
        assert_eq!(summary("error: line 1: not a Specctra design\n"), None);
    }
}
