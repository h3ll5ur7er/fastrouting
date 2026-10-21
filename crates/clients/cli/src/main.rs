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
    /// Route a board: a KiCad board file, or a design exported as Specctra DSN,
    /// whose routes go into a session (SES).
    Route {
        /// The board: BOARD.kicad_pcb, with the rules of BOARD.kicad_pro, or a
        /// Specctra design (KiCad: File → Export → Specctra DSN).
        #[arg(required_unless_present = "kicad")]
        input: Option<PathBuf>,
        /// Route the board open in KiCad's PCB editor instead, through KiCad's API
        /// (Preferences → Plugins → Enable KiCad API), and add the routes to it as
        /// one step that Edit → Undo takes back.
        #[arg(long, conflicts_with = "input")]
        kicad: bool,
        /// Where to write the routes. By default beside the input: BOARD.ses for a
        /// design, a copy BOARD.routed.kicad_pcb (and its project) for a KiCad board;
        /// into KiCad with --kicad.
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
        /// Write a DRC report of the routed board, in the JSON format of
        /// `kicad-cli pcb drc --format json`.
        #[arg(long, value_name = "REPORT.json")]
        drc: Option<PathBuf>,
    },
    /// Check a board: how many of its nets are complete and which rules its
    /// copper breaks, with the routes of a session if one is given.
    Check {
        /// The board: a KiCad board file, or a Specctra design.
        board: PathBuf,
        /// A session (SES) with routes for the board.
        session: Option<PathBuf>,
        /// Print the report as JSON.
        #[arg(long)]
        json: bool,
        /// Write a DRC report of the board, in the JSON format of
        /// `kicad-cli pcb drc --format json`.
        #[arg(long, value_name = "REPORT.json")]
        drc: Option<PathBuf>,
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
    let (input, output, profile, config, threads, quiet, drc) = match cli.command {
        Command::Freerouting { args } => return run_as_freerouting(&args),
        Command::Check {
            board,
            session,
            json,
            drc,
        } => {
            return run_check(
                CheckJob {
                    board,
                    session,
                    drc_report: drc,
                },
                json,
            );
        }
        Command::Route {
            input,
            // Clap lets through either a board or --kicad, never both.
            kicad: _,
            output,
            profile,
            config,
            threads,
            quiet,
            drc,
        } => {
            let input = input.unwrap_or_else(|| PathBuf::from(traxide_routing::RUNNING_KICAD));
            (input, output, profile, config, threads, quiet, drc)
        }
    };
    let job = RouteJob {
        input,
        output,
        profile,
        profile_file: config,
        threads,
        unrouted_net_classes: Vec::new(),
        drc_report: drc,
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
    Ok(summarise(&report))
}

/// Exits successfully only when every net is complete and no rule broken.
fn run_check(job: CheckJob, json: bool) -> Result<ExitCode> {
    let report = traxide_inspection::check(&job)?;
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
    summarise(&report);
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

fn summarise(report: &RouteReport) -> ExitCode {
    let routed = report.connections - report.unrouted.len();
    println!(
        "Routed {routed} of {} connections ({} tracks, {} vias) in {:.2} s → {}",
        report.connections,
        report.tracks,
        report.vias,
        report.elapsed.as_secs_f64(),
        report.output.display()
    );
    if !report.planes.is_empty() {
        println!(
            "Kept free of tracks, as planes: {}",
            report.planes.join(", ")
        );
    }
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
