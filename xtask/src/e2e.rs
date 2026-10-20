//! The real-world test: KiCad builds each board and exports it, traxide routes
//! it, KiCad imports the session and KiCad's own DRC judges the result.

use crate::kicad::{self, Kicad};
use anyhow::{Result, bail, ensure};
use std::path::Path;
use std::process::Command;
use std::time::Instant;

const DEFAULT_RECIPES: &[&str] = &["placements", "crossing", "matrix-3x4", "isolation"];

pub fn run(args: &[String]) -> Result<()> {
    let kicad = Kicad::from_args(args);
    let (root, traxide) = kicad::build_traxide()?;
    let mut failures = Vec::new();
    for recipe in kicad::recipes(args, DEFAULT_RECIPES) {
        match round_trip(&kicad, &root, &traxide, recipe) {
            Ok(summary) => println!("{recipe}: {summary}"),
            Err(error) => {
                println!("{recipe}: FAILED\n{error:#}");
                failures.push(recipe);
            }
        }
    }
    if !failures.is_empty() {
        bail!(
            "{} of the boards failed: {}",
            failures.len(),
            failures.join(", ")
        );
    }
    Ok(())
}

fn round_trip(kicad: &Kicad, root: &Path, traxide: &Path, recipe: &str) -> Result<String> {
    let out = root.join("target/e2e").join(recipe);
    std::fs::create_dir_all(&out)?;
    kicad.export(root, &out, recipe)?;
    // Specctra DSN has no words for KiCad's custom rules, so a board that has
    // them is routed from its own file only.
    let custom_rules = out.join(format!("{recipe}.kicad_dru")).is_file();
    let mut summary = String::new();
    if !custom_rules {
        summary = through_dsn(kicad, root, traxide, recipe, &out)?;
    }
    let started = Instant::now();
    let direct = Command::new(traxide)
        .arg("route")
        .arg("--quiet")
        .arg(out.join(format!("{recipe}.kicad_pcb")))
        .arg("-o")
        .arg(out.join(format!("{recipe}.direct.kicad_pcb")))
        .output()?;
    let seconds = started.elapsed().as_secs_f64();
    ensure!(
        direct.status.success(),
        "traxide did not route the KiCad board completely:\n{}{}",
        String::from_utf8_lossy(&direct.stdout),
        String::from_utf8_lossy(&direct.stderr)
    );
    let from_board = kicad.judge_board(root, &out, &format!("{recipe}.direct"))?;
    ensure!(
        from_board.is_clean(),
        "routed from the board file, KiCad's DRC found {} violations and {} unconnected items:\n  {}",
        from_board.violations.len(),
        from_board.unconnected.len(),
        [from_board.violations, from_board.unconnected]
            .concat()
            .join("\n  ")
    );
    if custom_rules {
        return Ok(format!(
            "routed from the board file by its custom rules in {seconds:.2} s, clean in KiCad's DRC ({})",
            first_line(&direct.stdout)
                .split(" → ")
                .next()
                .unwrap_or_default()
        ));
    }
    let through_plugin =
        kicad.route_with_plugin(root, &out, recipe, &kicad.traxide_for_kicad(root)?)?;
    ensure!(
        through_plugin.is_clean(),
        "routed through the KiCad plugin, KiCad's DRC found {} violations and {} unconnected items:\n  {}",
        through_plugin.violations.len(),
        through_plugin.unconnected.len(),
        [through_plugin.violations, through_plugin.unconnected]
            .concat()
            .join("\n  ")
    );
    Ok(format!(
        "clean in KiCad's DRC, from the board file and through the KiCad plugin too; {summary}"
    ))
}

/// Routes the recipe's DSN export and has KiCad judge the imported session;
/// says how long routing took.
fn through_dsn(
    kicad: &Kicad,
    root: &Path,
    traxide: &Path,
    recipe: &str,
    out: &Path,
) -> Result<String> {
    let started = Instant::now();
    let routed = Command::new(traxide)
        .arg("route")
        .arg("--quiet")
        .arg(out.join(format!("{recipe}.dsn")))
        .output()?;
    let seconds = started.elapsed().as_secs_f64();
    ensure!(
        routed.status.success(),
        "traxide did not route everything:\n{}",
        String::from_utf8_lossy(&routed.stdout)
    );
    let judgement = kicad.judge(
        root,
        out,
        recipe,
        &format!("{recipe}.ses"),
        &format!("{recipe}.routed"),
    )?;
    ensure!(
        judgement.is_clean(),
        "KiCad's DRC found {} violations and {} unconnected items:\n  {}",
        judgement.violations.len(),
        judgement.unconnected.len(),
        [judgement.violations, judgement.unconnected]
            .concat()
            .join("\n  ")
    );
    Ok(format!(
        "routed in {seconds:.2} s ({})",
        first_line(&routed.stdout)
            .split(" → ")
            .next()
            .unwrap_or_default()
    ))
}

fn first_line(output: &[u8]) -> String {
    String::from_utf8_lossy(output)
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned()
}
