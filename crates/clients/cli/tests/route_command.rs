//! End to end: the `traxide` binary routes a board, and what it writes is read
//! back and checked independently.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use traxide_rules::Purpose;

fn board(name: &str) -> PathBuf {
    PathBuf::from(format!(
        "{}/../../../fixtures/boards/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
}

fn traxide(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_traxide"))
        .args(args)
        .output()
        .expect("the binary runs")
}

fn copy_to(folder: &Path, name: &str) -> PathBuf {
    let copy = folder.join(name);
    std::fs::copy(board(name), &copy).expect("copied");
    copy
}

#[test]
fn routes_a_board_and_writes_a_session_next_to_it() {
    let folder = tempfile::tempdir().expect("temp dir");
    let input = copy_to(folder.path(), "two-nets.dsn");
    let run = traxide(&["route", input.to_str().expect("utf-8"), "--quiet"]);
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        run.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(stdout.contains("Routed 2 of 2 connections"), "{stdout}");

    let design = traxide_board_access::load_design(&input).expect("valid board");
    let routes = traxide_board_access::load_routes(&input.with_extension("ses"), &design)
        .expect("valid session");
    let rules = traxide_rules::compile(&design, Purpose::Verification);
    let verdict = traxide_verification::verify(&design, &rules, &routes);
    assert!(verdict.is_clean(), "{verdict:?}");
}

#[test]
fn writes_the_same_session_on_any_number_of_threads() {
    let folder = tempfile::tempdir().expect("temp dir");
    // Exported by KiCad 10 from the matrix-3x4 recipe of e2e/kicad/boards.py.
    let input = copy_to(folder.path(), "matrix-3x4.dsn");
    let session_on = |threads: &str| {
        // The same file name each time, as a session names itself after its file.
        let output = folder.path().join(threads).join("routed.ses");
        std::fs::create_dir(output.parent().expect("in a folder")).expect("new folder");
        let run = traxide(&[
            "route",
            input.to_str().expect("utf-8"),
            "-o",
            output.to_str().expect("utf-8"),
            "--threads",
            threads,
            "--quiet",
        ]);
        assert!(run.status.success(), "{run:?}");
        std::fs::read(output).expect("written")
    };
    let on_one = session_on("1");
    assert_eq!(session_on("4"), on_one);
    assert_eq!(session_on("16"), on_one);
}

#[test]
fn an_unknown_profile_fails_with_the_known_ones() {
    let folder = tempfile::tempdir().expect("temp dir");
    let input = copy_to(folder.path(), "two-nets.dsn");
    let run = traxide(&[
        "route",
        input.to_str().expect("utf-8"),
        "--profile",
        "turbo",
    ]);
    assert_eq!(run.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&run.stderr).contains("no profile named turbo; there are default")
    );
}

#[test]
fn a_missing_board_fails_and_says_which() {
    let run = traxide(&["route", "no-such-board.dsn"]);
    assert_eq!(run.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&run.stderr).contains("no-such-board.dsn"));
}
