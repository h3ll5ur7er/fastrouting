//! The `traxide` command line.

mod freerouting;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::io::Write;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use traxide_contracts::{Event, Stage, Violation};
use traxide_events::CancelToken;
use traxide_inspection::CheckJob;
use traxide_routing::{RouteJob, RouteReport, Unroutable};

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
    /// Check a board: how many of its nets are complete and which rules its
    /// copper breaks, with the routes of a session if one is given.
    Check {
        /// The board, as Specctra DSN.
        board: PathBuf,
        /// A session (SES) with routes for the board.
        session: Option<PathBuf>,
        /// Print the report as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Take Freerouting's arguments (-de board.dsn -do board.ses …), for tools
    /// that run Freerouting. A binary named `freerouting` does the same.
    #[command(disable_help_flag = true)]
    Freerouting {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 0..)]
        args: Vec<String>,
    },
}

/// Exit codes beyond success: scripts tell an incomplete board from a failure.
const FAILED: u8 = 1;
const INCOMPLETE: u8 = 2;
const BROKE_A_RULE: u8 = 3;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let called_as_freerouting = args
        .first()
        .and_then(|program| Path::new(program).file_stem())
        .is_some_and(|name| name == "freerouting");
    let outcome = if called_as_freerouting {
        run_as_freerouting(&args[1..])
    } else {
        run(Cli::parse_from(args))
    };
    match outcome {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::from(FAILED)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    let (input, output, profile, config, threads, quiet) = match cli.command {
        Command::Freerouting { args } => return run_as_freerouting(&args),
        Command::Check {
            board,
            session,
            json,
        } => return run_check(board, session, json),
        Command::Route {
            input,
            output,
            profile,
            config,
            threads,
            quiet,
        } => (input, output, profile, config, threads, quiet),
    };
    let output = output.unwrap_or_else(|| input.with_extension("ses"));
    let job = RouteJob {
        input,
        output: output.clone(),
        profile,
        profile_file: config,
        threads,
        unrouted_net_classes: Vec::new(),
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

/// Exits successfully only when every net is complete and no rule broken.
fn run_check(board: PathBuf, session: Option<PathBuf>, json: bool) -> Result<ExitCode> {
    let report = traxide_inspection::check(&CheckJob { board, session })?;
    let kinds = report.violations_by_kind();
    if json {
        let violations: serde_json::Map<String, serde_json::Value> = kinds
            .iter()
            .map(|(kind, count)| ((*kind).to_owned(), (*count).into()))
            .collect();
        println!(
            "{}",
            serde_json::json!({
                "nets": report.nets,
                "incomplete_nets": report.incomplete_nets,
                "unconnected_pads": report.unconnected_pads,
                "violations": violations,
                "tracks": report.tracks,
                "vias": report.vias,
                "track_length_mm": report.track_length_mm,
            })
        );
    } else {
        println!(
            "{} of {} nets complete, {} pads unconnected; {} tracks, {} vias, {:.0} mm of track.",
            report.nets - report.incomplete_nets,
            report.nets,
            report.unconnected_pads,
            report.tracks,
            report.vias,
            report.track_length_mm
        );
        if kinds.is_empty() {
            println!("No rule broken.");
        } else {
            let counts: Vec<String> = kinds.iter().map(|(k, n)| format!("{n} {k}")).collect();
            println!("Rules broken: {}.", counts.join(", "));
        }
    }
    Ok(if !kinds.is_empty() {
        ExitCode::from(BROKE_A_RULE)
    } else if report.incomplete_nets > 0 {
        ExitCode::from(INCOMPLETE)
    } else {
        ExitCode::SUCCESS
    })
}

/// Exits successfully whenever a session was written, even an incomplete one:
/// tools that run Freerouting import the session after a successful exit.
fn run_as_freerouting(args: &[String]) -> Result<ExitCode> {
    let (job, warnings) = match freerouting::parse(args)? {
        freerouting::Invocation::Help => {
            println!("{}", freerouting::HELP);
            return Ok(ExitCode::SUCCESS);
        }
        freerouting::Invocation::Route { job, warnings } => (job, warnings),
    };
    for warning in warnings {
        eprintln!("warning: {warning}");
    }
    let report = traxide_routing::route(&job, &|_: Event| {}, &CancelToken::new())?;
    summarise(&report, &job.output);
    Ok(ExitCode::SUCCESS)
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
    for unrouted in &report.unrouted {
        let why = match &unrouted.why {
            Unroutable::OffTheBoard(pad) => format!("{pad} lies outside the board"),
            Unroutable::OnlyOnLayersNotRouted(pad) => {
                format!("{pad} is only on layers not routed on, such as planes")
            }
            Unroutable::HemmedIn(pad) => format!(
                "no track can leave {pad}: another net's copper, a keepout or the edge is too close"
            ),
            Unroutable::NoWayFound => "no way found".to_owned(),
        };
        println!(
            "  unrouted: {} from {} to {}: {why}",
            unrouted.net, unrouted.from, unrouted.to
        );
    }
    if report.violations_before_routing > 0 {
        println!(
            "Note: the board broke {} rules before routing, in the copper it came with; they are as they were.",
            report.violations_before_routing
        );
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
