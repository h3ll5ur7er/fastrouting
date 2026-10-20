//! Routes boards with traxide and, when given, with Freerouting, and has
//! KiCad judge both alike: `cargo xtask bench [--freerouting "java -jar
//! freerouting.jar"] [--timeout SECONDS] [RECIPE…]`. Prints a Markdown table,
//! and appends it to `$GITHUB_STEP_SUMMARY` in CI.

use crate::kicad::{self, Judgement, Kicad};
use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const DEFAULT_RECIPES: &[&str] = &["matrix-6x17", "keyboard-tkl"];

struct Run {
    board: String,
    router: &'static str,
    seconds: f64,
    finished: bool,
    judgement: Option<Judgement>,
}

pub fn run(args: &[String]) -> Result<()> {
    let kicad = Kicad::from_args(args);
    let freerouting = option(args, "--freerouting");
    let timeout = Duration::from_secs(option(args, "--timeout").map_or(Ok(3600), |t| t.parse())?);
    let (root, traxide) = kicad::build_traxide()?;
    let named = without_options(args);
    let recipes = kicad::recipes(&named, DEFAULT_RECIPES);
    let mut runs = Vec::new();
    for recipe in recipes {
        let out = root.join("target/bench").join(recipe);
        std::fs::create_dir_all(&out)?;
        kicad.export(&root, &out, recipe)?;
        let dsn = out.join(format!("{recipe}.dsn"));
        let session = format!("{recipe}.traxide.ses");
        let mut command = Command::new(&traxide);
        command
            .arg("route")
            .arg("--quiet")
            .arg(&dsn)
            .arg("-o")
            .arg(out.join(&session));
        runs.push(measure(
            &kicad, &root, &out, recipe, "traxide", command, &session, timeout,
        )?);
        if let Some(freerouting) = &freerouting {
            let session = format!("{recipe}.freerouting.ses");
            let mut words = freerouting.split_whitespace();
            let mut command = Command::new(words.next().context("--freerouting names no command")?);
            command
                .args(words)
                .arg("-de")
                .arg(&dsn)
                .arg("-do")
                .arg(out.join(&session))
                .arg("--gui.enabled=false");
            runs.push(measure(
                &kicad,
                &root,
                &out,
                recipe,
                "Freerouting",
                command,
                &session,
                timeout,
            )?);
        }
    }
    let table = table(&runs);
    println!("{table}");
    if let Ok(summary) = std::env::var("GITHUB_STEP_SUMMARY") {
        std::fs::write(summary, format!("## Benchmark\n\n{table}\n"))?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn measure(
    kicad: &Kicad,
    root: &Path,
    out: &Path,
    recipe: &str,
    router: &'static str,
    mut command: Command,
    session: &str,
    timeout: Duration,
) -> Result<Run> {
    let started = Instant::now();
    let mut child = command
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context(router)?;
    let finished = loop {
        if child.try_wait()?.is_some() {
            break true;
        }
        if started.elapsed() > timeout {
            child.kill()?;
            break false;
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let seconds = started.elapsed().as_secs_f64();
    let routed_name = session.trim_end_matches(".ses").to_owned() + ".routed";
    let judgement = if out.join(session).exists() {
        Some(kicad.judge(root, out, recipe, session, &routed_name)?)
    } else {
        None
    };
    Ok(Run {
        board: recipe.into(),
        router,
        seconds,
        finished,
        judgement,
    })
}

fn table(runs: &[Run]) -> String {
    let mut rows = vec![
        "| Board | Router | Time | Unconnected | Violations | Tracks | Vias | Track length |"
            .to_owned(),
        "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |".to_owned(),
    ];
    for run in runs {
        let time = if run.finished {
            format!("{:.1} s", run.seconds)
        } else {
            format!("> {:.0} s (stopped)", run.seconds)
        };
        rows.push(match &run.judgement {
            Some(j) => format!(
                "| {} | {} | {time} | {} | {} | {} | {} | {:.0} mm |",
                run.board,
                run.router,
                j.unconnected.len(),
                j.violations.len(),
                j.tracks,
                j.vias,
                j.length_mm
            ),
            None => format!(
                "| {} | {} | {time} | no session | | | | |",
                run.board, run.router
            ),
        });
    }
    rows.join("\n")
}

fn option(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

/// The arguments without options and their values, which leaves the recipes.
fn without_options(args: &[String]) -> Vec<String> {
    let mut rest = Vec::new();
    let mut skip = false;
    for arg in args {
        if skip {
            skip = false;
        } else if arg == "--freerouting" || arg == "--timeout" {
            skip = true;
        } else {
            rest.push(arg.clone());
        }
    }
    rest
}
