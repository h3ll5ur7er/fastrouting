//! Compares traxide's corpus results with the stored baseline:
//! `cargo xtask gate [--baseline FILE] [--partial] [--update-baseline]
//! [--merged FILE] RESULTS.json…`. Fails when a board the baseline completes is no longer
//! complete, when any board breaks a rule, or when a board routes clearly
//! fewer nets; lists the improvements, so that the baseline can be raised
//! with `--update-baseline`, or by copying the file `--merged` writes, all
//! the results in one, over the baseline. `--partial` compares only the boards the results
//! hold, as after one shard of the corpus.

use crate::corpus::negotiation_limit;
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::time::Duration;

const DEFAULT_BASELINE: &str = "corpus/baseline.json";

/// traxide's results on a set of boards, as `cargo xtask corpus` writes them
/// and as the baseline keeps them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub timeout_seconds: u64,
    pub boards: BTreeMap<String, Outcome>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "lowercase")]
pub enum Outcome {
    Finished {
        seconds: f64,
        nets: usize,
        routed: usize,
        rules_broken: usize,
    },
    Stopped {
        seconds: f64,
    },
    Failed {
        why: String,
    },
}

/// What the comparison found: regressions fail the gate.
#[derive(Debug, Default)]
pub struct Verdict {
    pub regressions: Vec<String>,
    pub improvements: Vec<String>,
    pub notes: Vec<String>,
}

pub fn run(args: &[String]) -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()?;
    let mut baseline_file = root.join(DEFAULT_BASELINE);
    let (mut partial, mut update) = (false, false);
    let mut merged_file = None;
    let mut result_files = Vec::new();
    let mut words = args.iter();
    while let Some(word) = words.next() {
        match word.as_str() {
            "--baseline" => {
                baseline_file = words.next().context("--baseline names no file")?.into();
            }
            "--merged" => {
                merged_file = Some(PathBuf::from(
                    words.next().context("--merged names no file")?,
                ));
            }
            "--partial" => partial = true,
            "--update-baseline" => update = true,
            file => result_files.push(PathBuf::from(file)),
        }
    }
    ensure!(
        !result_files.is_empty(),
        "gate: name the results to compare"
    );
    let mut records = Vec::new();
    for file in &result_files {
        let text = std::fs::read_to_string(file).with_context(|| file.display().to_string())?;
        records.push(serde_json::from_str(&text).with_context(|| file.display().to_string())?);
    }
    let results = merged(records)?;
    if let Some(file) = &merged_file {
        std::fs::write(file, serde_json::to_string_pretty(&results)? + "\n")?;
    }
    let baseline: Record = serde_json::from_str(
        &std::fs::read_to_string(&baseline_file)
            .with_context(|| baseline_file.display().to_string())?,
    )?;
    let verdict = compare(&baseline, &results, partial)?;
    let report = format!(
        "{}\n{}",
        verdict.markdown(),
        table(&baseline, &results, &verdict)
    );
    println!("{report}");
    if let Ok(summary) = std::env::var("GITHUB_STEP_SUMMARY") {
        std::fs::write(summary, format!("## Corpus gate\n\n{report}\n"))?;
    }
    if update {
        let raised = if partial {
            Record {
                boards: baseline.boards.into_iter().chain(results.boards).collect(),
                ..results
            }
        } else {
            results
        };
        std::fs::write(
            &baseline_file,
            serde_json::to_string_pretty(&raised)? + "\n",
        )?;
        eprintln!("wrote {}", baseline_file.display());
    } else if !verdict.regressions.is_empty() {
        bail!(
            "{} regressions against {}",
            verdict.regressions.len(),
            baseline_file.display()
        );
    }
    Ok(())
}

/// The results of several shards as one record.
pub fn merged(records: Vec<Record>) -> Result<Record> {
    let mut records = records.into_iter();
    let mut all = records.next().context("no results")?;
    for record in records {
        ensure!(
            record.timeout_seconds == all.timeout_seconds,
            "results with time limits of {} s and {} s",
            all.timeout_seconds,
            record.timeout_seconds
        );
        all.boards.extend(record.boards);
    }
    Ok(all)
}

impl Outcome {
    fn is_complete(&self) -> bool {
        matches!(self, Outcome::Finished { nets, routed, rules_broken: 0, .. } if routed == nets)
    }

    pub fn describe(&self) -> String {
        match self {
            Outcome::Finished {
                seconds,
                nets,
                routed,
                rules_broken,
            } => format!("{seconds:.1} s, {routed} of {nets} nets, {rules_broken} rules broken"),
            Outcome::Stopped { seconds } => format!("stopped after {seconds:.0} s"),
            Outcome::Failed { why } => format!("failed: {why}"),
        }
    }
}

/// Nets a board may route fewer than the baseline before the gate fails:
/// 2 % of them, as a change to the router trades a net here for one there.
fn nets_allowed_lost(nets: usize) -> usize {
    nets / 50
}

/// A board the baseline finishes in at least half the time limit may stop
/// at the limit on a slower runner with the same routes.
fn near_the_limit(baseline: &Outcome, timeout_seconds: u64) -> bool {
    match baseline {
        Outcome::Finished { seconds, .. } => seconds * 2.0 >= timeout_seconds as f64,
        _ => false,
    }
}

/// A board that took the corpus's negotiation limit, in the baseline or now,
/// was routed against the clock: how many nets it routes then depends on the
/// runner's speed, by up to a tenth of them on the same commit.
fn against_the_clock(seconds: [f64; 2], timeout_seconds: u64) -> bool {
    let limit = negotiation_limit(Duration::from_secs(timeout_seconds)).as_secs_f64();
    seconds.iter().any(|&s| s >= limit)
}

/// Clearly slower: more than twice the baseline's time and 10 s more.
fn clearly_slower(seconds: f64, baseline_seconds: f64) -> bool {
    seconds > 2.0 * baseline_seconds && seconds > baseline_seconds + 10.0
}

pub fn compare(baseline: &Record, results: &Record, partial: bool) -> Result<Verdict> {
    ensure!(
        baseline.timeout_seconds == results.timeout_seconds,
        "the results ran with a time limit of {} s, the baseline with {} s",
        results.timeout_seconds,
        baseline.timeout_seconds
    );
    let mut verdict = Verdict::default();
    for (board, now) in &results.boards {
        if let Outcome::Finished {
            rules_broken: broken @ 1..,
            ..
        } = now
        {
            verdict
                .regressions
                .push(format!("{board} breaks {broken} rules"));
        }
        let Some(before) = baseline.boards.get(board) else {
            verdict.notes.push(format!(
                "{board} is not in the baseline: {}",
                now.describe()
            ));
            continue;
        };
        compare_board(board, before, now, results.timeout_seconds, &mut verdict);
    }
    if !partial {
        for board in baseline.boards.keys() {
            if !results.boards.contains_key(board) {
                verdict
                    .regressions
                    .push(format!("{board} is missing from the results"));
            }
        }
    }
    Ok(verdict)
}

fn compare_board(
    board: &str,
    before: &Outcome,
    now: &Outcome,
    timeout_seconds: u64,
    verdict: &mut Verdict,
) {
    let against = format!("{}, baseline {}", now.describe(), before.describe());
    match (before, now) {
        (
            Outcome::Finished {
                seconds: seconds_before,
                routed: routed_before,
                ..
            },
            Outcome::Finished {
                seconds,
                nets,
                routed,
                ..
            },
        ) => {
            if before.is_complete() && !now.is_complete() {
                verdict
                    .regressions
                    .push(format!("{board} is no longer complete: {against}"));
            } else if routed + nets_allowed_lost(*nets) < *routed_before
                && !against_the_clock([*seconds_before, *seconds], timeout_seconds)
            {
                verdict
                    .regressions
                    .push(format!("{board} routes fewer nets: {against}"));
            } else if routed > routed_before {
                verdict
                    .improvements
                    .push(format!("{board} routes more nets: {against}"));
            } else if routed < routed_before {
                let how = if against_the_clock([*seconds_before, *seconds], timeout_seconds) {
                    "fewer nets against the clock"
                } else {
                    "a few nets fewer"
                };
                verdict
                    .notes
                    .push(format!("{board} routes {how}: {against}"));
            }
            if clearly_slower(*seconds, *seconds_before) {
                verdict
                    .notes
                    .push(format!("{board} is clearly slower: {against}"));
            }
        }
        (Outcome::Finished { .. }, Outcome::Stopped { .. })
            if near_the_limit(before, timeout_seconds) =>
        {
            verdict.notes.push(format!(
                "{board} stopped, but the baseline finishes near the limit: {against}"
            ));
        }
        (Outcome::Finished { .. }, _) => {
            verdict
                .regressions
                .push(format!("{board} no longer finishes: {against}"));
        }
        (Outcome::Stopped { .. }, Outcome::Finished { .. }) => {
            verdict
                .improvements
                .push(format!("{board} finishes: {against}"));
        }
        (Outcome::Stopped { .. }, Outcome::Failed { .. }) => {
            verdict
                .regressions
                .push(format!("{board} fails: {against}"));
        }
        (Outcome::Failed { .. }, Outcome::Finished { .. } | Outcome::Stopped { .. }) => {
            verdict
                .improvements
                .push(format!("{board} no longer fails: {against}"));
        }
        (Outcome::Stopped { .. }, Outcome::Stopped { .. })
        | (Outcome::Failed { .. }, Outcome::Failed { .. }) => {}
    }
}

impl Verdict {
    pub fn markdown(&self) -> String {
        let mut text = String::new();
        let _ = writeln!(
            text,
            "{} regressions, {} improvements.",
            self.regressions.len(),
            self.improvements.len()
        );
        for (heading, lines) in [
            ("Regressions", &self.regressions),
            (
                "Improvements, to raise the baseline with",
                &self.improvements,
            ),
            ("Notes", &self.notes),
        ] {
            if !lines.is_empty() {
                let _ = writeln!(text, "\n**{heading}**\n");
                for line in lines {
                    let _ = writeln!(text, "- {line}");
                }
            }
        }
        text
    }
}

/// Every board, now and in the baseline, with the regressions marked.
fn table(baseline: &Record, results: &Record, verdict: &Verdict) -> String {
    let regressed = |board: &str| {
        verdict
            .regressions
            .iter()
            .any(|line| line.split_whitespace().next() == Some(board))
    };
    let mut lines = vec!["| Board | traxide | Baseline |\n| --- | --- | --- |".to_owned()];
    for (board, now) in &results.boards {
        let mark = if regressed(board) {
            " **regressed**"
        } else {
            ""
        };
        let before = baseline
            .boards
            .get(board)
            .map_or_else(|| "none".to_owned(), Outcome::describe);
        lines.push(format!("| {board}{mark} | {} | {before} |", now.describe()));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finished(seconds: f64, nets: usize, routed: usize) -> Outcome {
        Outcome::Finished {
            seconds,
            nets,
            routed,
            rules_broken: 0,
        }
    }

    fn record(boards: &[(&str, Outcome)]) -> Record {
        Record {
            timeout_seconds: 120,
            boards: boards
                .iter()
                .map(|(b, o)| ((*b).to_owned(), o.clone()))
                .collect(),
        }
    }

    fn verdict(before: Outcome, now: Outcome) -> Verdict {
        compare(&record(&[("b", before)]), &record(&[("b", now)]), false).unwrap()
    }

    #[test]
    fn the_same_results_pass() {
        let v = verdict(finished(3.0, 10, 10), finished(3.2, 10, 10));
        assert!(v.regressions.is_empty() && v.improvements.is_empty());
    }

    #[test]
    fn a_board_the_baseline_completes_must_stay_complete() {
        let v = verdict(finished(3.0, 100, 100), finished(3.0, 100, 99));
        assert_eq!(v.regressions.len(), 1, "{v:?}");
        assert!(v.regressions[0].starts_with("b is no longer complete"));
    }

    #[test]
    fn a_complete_board_that_stops_far_below_the_limit_regresses() {
        let v = verdict(finished(20.0, 10, 10), Outcome::Stopped { seconds: 120.0 });
        assert_eq!(v.regressions.len(), 1, "{v:?}");
    }

    #[test]
    fn a_board_the_baseline_finishes_near_the_limit_may_stop() {
        let v = verdict(finished(108.0, 10, 10), Outcome::Stopped { seconds: 120.0 });
        assert!(v.regressions.is_empty(), "{v:?}");
        assert_eq!(v.notes.len(), 1);
    }

    #[test]
    fn any_broken_rule_fails() {
        let broken = Outcome::Finished {
            seconds: 1.0,
            nets: 10,
            routed: 8,
            rules_broken: 1,
        };
        let v = verdict(finished(1.0, 10, 8), broken.clone());
        assert_eq!(v.regressions, ["b breaks 1 rules"]);
        let new = compare(&record(&[]), &record(&[("b", broken)]), false).unwrap();
        assert_eq!(new.regressions, ["b breaks 1 rules"]);
    }

    #[test]
    fn routing_clearly_fewer_nets_fails_but_a_few_fewer_is_noted() {
        assert_eq!(
            verdict(finished(1.0, 100, 90), finished(1.0, 100, 87))
                .regressions
                .len(),
            1
        );
        let v = verdict(finished(1.0, 100, 90), finished(1.0, 100, 88));
        assert!(v.regressions.is_empty());
        assert_eq!(v.notes.len(), 1);
    }

    #[test]
    fn a_board_routed_against_the_clock_may_route_fewer_nets_on_another_runner() {
        // traxide negotiates for 90 of the corpus's 120 s, so how far it gets
        // there depends on the runner's speed.
        for (before, now) in [
            (finished(91.0, 415, 236), finished(92.0, 415, 227)),
            (finished(60.0, 415, 236), finished(92.0, 415, 227)),
            (finished(92.0, 415, 236), finished(60.0, 415, 227)),
        ] {
            let v = verdict(before, now);
            assert!(v.regressions.is_empty(), "{v:?}");
            assert_eq!(v.notes.len(), 1, "{v:?}");
        }
        let complete_before = verdict(finished(91.0, 9, 9), finished(91.0, 9, 8));
        assert_eq!(complete_before.regressions.len(), 1);
    }

    #[test]
    fn improvements_are_listed() {
        let more = verdict(finished(1.0, 100, 90), finished(1.0, 100, 95));
        let finishes = verdict(Outcome::Stopped { seconds: 120.0 }, finished(80.0, 9, 9));
        let no_longer_fails = verdict(Outcome::Failed { why: "x".into() }, finished(80.0, 9, 9));
        for v in [more, finishes, no_longer_fails] {
            assert!(v.regressions.is_empty(), "{v:?}");
            assert_eq!(v.improvements.len(), 1, "{v:?}");
        }
    }

    #[test]
    fn failing_where_the_baseline_routed_regresses() {
        let fails = Outcome::Failed { why: "x".into() };
        assert_eq!(
            verdict(finished(1.0, 9, 5), fails.clone())
                .regressions
                .len(),
            1
        );
        assert_eq!(
            verdict(Outcome::Stopped { seconds: 120.0 }, fails)
                .regressions
                .len(),
            1
        );
    }

    #[test]
    fn a_board_missing_from_the_results_fails_unless_partial() {
        let baseline = record(&[("a", finished(1.0, 9, 9)), ("b", finished(1.0, 9, 9))]);
        let results = record(&[("a", finished(1.0, 9, 9))]);
        assert_eq!(
            compare(&baseline, &results, false).unwrap().regressions,
            ["b is missing from the results"]
        );
        assert!(
            compare(&baseline, &results, true)
                .unwrap()
                .regressions
                .is_empty()
        );
    }

    #[test]
    fn results_with_another_time_limit_are_not_compared() {
        let mut results = record(&[]);
        results.timeout_seconds = 60;
        assert!(compare(&record(&[]), &results, false).is_err());
    }

    #[test]
    fn shards_merge_into_one_record() {
        let all = merged(vec![
            record(&[("a", finished(1.0, 9, 9))]),
            record(&[("b", Outcome::Stopped { seconds: 120.0 })]),
        ])
        .unwrap();
        assert_eq!(all.boards.len(), 2);
    }

    #[test]
    fn a_record_reads_back_what_it_wrote() {
        let written = record(&[
            ("a", finished(1.5, 9, 9)),
            ("b", Outcome::Stopped { seconds: 120.0 }),
            ("c", Outcome::Failed { why: "no".into() }),
        ]);
        let json = serde_json::to_string(&written).unwrap();
        assert!(json.contains(r#""outcome":"finished""#), "{json}");
        assert_eq!(serde_json::from_str::<Record>(&json).unwrap(), written);
    }
}
