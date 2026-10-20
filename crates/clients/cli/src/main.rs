//! The `traxide` command line.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::io::Write;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::process::ExitCode;
use traxide_contracts::{Event, Stage, Violation};
use traxide_events::CancelToken;
use traxide_routing::{RouteJob, RouteReport};

/// A fast PCB autorouter.
#[derive(Debug, Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Route a board exported as Specctra DSN and write the routes as a session (SES).
    Route {
        /// The board, such as one exported by KiCad (File → Export → Specctra DSN).
        input: PathBuf,
        /// Where to write the session; next to the input, as .ses, by default.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// The routing profile.
        #[arg(long, default_value = "default")]
        profile: String,
        /// A traxide.toml with profiles of your own.
        #[arg(long)]
        config: Option<PathBuf>,
        /// How many threads to route on; all cores by default. The routes are the
        /// same on any number of them.
        #[arg(short = 'j', long)]
        threads: Option<NonZeroUsize>,
        /// Print only the summary.
        #[arg(short, long)]
        quiet: bool,
    },
}

/// Exit codes beyond success: scripts tell an incomplete board from a failure.
const FAILED: u8 = 1;
const INCOMPLETE: u8 = 2;
const BROKE_A_RULE: u8 = 3;

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::from(FAILED)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    let Command::Route {
        input,
        output,
        profile,
        config,
        threads,
        quiet,
    } = cli.command;
    let output = output.unwrap_or_else(|| input.with_extension("ses"));
    let job = RouteJob {
        input,
        output: output.clone(),
        profile,
        profile_file: config,
        threads,
    };
    let progress = |event: Event| {
        if !quiet {
            show(&event);
        }
    };
    let report = traxide_routing::route(&job, &progress, &CancelToken::new())?;
    if !quiet {
        eprintln!();
    }
    Ok(summarise(&report, &output))
}

fn show(event: &Event) {
    let mut stderr = std::io::stderr();
    // Progress is a courtesy; a closed stderr must not stop the routing.
    let _ = match event {
        Event::StageStarted(Stage::Routing) => write!(stderr, "Routing"),
        Event::StageStarted(stage) if *stage != Stage::Loading => {
            write!(stderr, "\r\x1b[K{stage:?}…")
        }
        Event::StageStarted(_) => Ok(()),
        Event::ConnectionRouted { done, total } => write!(stderr, "\r\x1b[KRouting {done}/{total}"),
    };
}

fn summarise(report: &RouteReport, output: &std::path::Path) -> ExitCode {
    let routed = report.connections - report.unrouted.len();
    println!(
        "Routed {routed} of {} connections ({} tracks, {} vias) in {:.2} s → {}",
        report.connections,
        report.tracks,
        report.vias,
        report.elapsed.as_secs_f64(),
        output.display()
    );
    for (net, from, to) in &report.unrouted {
        println!("  unrouted: {net} from {from} to {to}");
    }
    let broken: Vec<&Violation> = report
        .verdict
        .violations
        .iter()
        .filter(|v| !matches!(v, Violation::Unconnected { .. }))
        .collect();
    if !broken.is_empty() {
        println!(
            "The routes break {} rules. This is a bug in traxide; please report it with the board:",
            broken.len()
        );
        for violation in broken {
            println!("  {violation:?}");
        }
        return ExitCode::from(BROKE_A_RULE);
    }
    if report.cancelled || !report.unrouted.is_empty() {
        ExitCode::from(INCOMPLETE)
    } else {
        ExitCode::SUCCESS
    }
}
