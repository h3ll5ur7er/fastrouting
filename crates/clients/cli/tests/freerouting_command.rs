//! End to end: tools that run Freerouting can run traxide with the same arguments.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use traxide_rules::Purpose;

fn board(name: &str) -> PathBuf {
    PathBuf::from(format!(
        "{}/../../../fixtures/boards/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
}

fn run(program: &Path, args: &[&str]) -> Output {
    Command::new(program)
        .args(args)
        .output()
        .expect("the binary runs")
}

/// The session at `session` joins every net of `input` and breaks no rule.
fn assert_complete_and_clean(input: &Path, session: &Path) {
    let design = traxide_board_access::load_design(input).expect("valid board");
    let routes = traxide_board_access::load_routes(session, &design).expect("valid session");
    let rules = traxide_rules::compile(&design, Purpose::Verification);
    let verdict = traxide_verification::verify(&design, &rules, &routes);
    assert!(verdict.is_clean(), "{verdict:?}");
}

#[test]
fn routes_with_freeroutings_arguments_and_warns_about_those_it_ignores() {
    let folder = tempfile::tempdir().expect("temp dir");
    let session = folder.path().join("routed.ses");
    let input = board("two-nets.dsn");
    let routed = run(
        Path::new(env!("CARGO_BIN_EXE_traxide")),
        &[
            "freerouting",
            "-de",
            input.to_str().expect("utf-8"),
            "-do",
            session.to_str().expect("utf-8"),
            "-mp",
            "20",
            "--gui.enabled=false",
        ],
    );
    assert!(routed.status.success(), "{routed:?}");
    let stderr = String::from_utf8_lossy(&routed.stderr);
    assert!(stderr.contains("warning: -mp: ignored"), "{stderr}");
    assert_complete_and_clean(&input, &session);
}

#[test]
fn a_binary_named_freerouting_takes_freeroutings_arguments() {
    let folder = tempfile::tempdir().expect("temp dir");
    let freerouting = folder
        .path()
        .join("freerouting")
        .with_extension(std::env::consts::EXE_EXTENSION);
    std::fs::copy(env!("CARGO_BIN_EXE_traxide"), &freerouting).expect("copied");
    let session = folder.path().join("routed.ses");
    let input = board("two-nets.dsn");
    let routed = run(
        &freerouting,
        &[
            "-de",
            input.to_str().expect("utf-8"),
            "-do",
            session.to_str().expect("utf-8"),
        ],
    );
    assert!(routed.status.success(), "{routed:?}");
    assert_complete_and_clean(&input, &session);
}

#[test]
fn drc_writes_a_drc_report_in_kicads_format_without_a_warning() {
    let folder = tempfile::tempdir().expect("temp dir");
    let session = folder.path().join("routed.ses");
    let drc = folder.path().join("routed.drc.json");
    let routed = run(
        Path::new(env!("CARGO_BIN_EXE_traxide")),
        &[
            "freerouting",
            "-de",
            board("two-nets.dsn").to_str().expect("utf-8"),
            "-do",
            session.to_str().expect("utf-8"),
            "-drc",
            drc.to_str().expect("utf-8"),
        ],
    );
    assert!(routed.status.success(), "{routed:?}");
    let stderr = String::from_utf8_lossy(&routed.stderr);
    assert!(!stderr.contains("warning"), "{stderr}");
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(drc).expect("written")).expect("JSON");
    assert_eq!(report["$schema"], "https://schemas.kicad.org/drc.v1.json");
    assert_eq!(report["source"], "routed.ses");
    assert_eq!(report["violations"], serde_json::json!([]));
    assert_eq!(report["unconnected_items"], serde_json::json!([]));
}
