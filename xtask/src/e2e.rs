//! The real-world test: KiCad builds each board and exports it, traxide routes
//! it, KiCad imports the session and KiCad's own DRC judges the result.

use crate::kicad::{self, Kicad};
use anyhow::{Result, bail, ensure};
use std::path::Path;
use std::process::Command;
use std::time::Instant;

const DEFAULT_RECIPES: &[&str] = &["placements", "crossing", "matrix-3x4"];

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
    let started = Instant::now();
    let routed = Command::new(traxide)
        .arg("route")
        .arg("--quiet")
        .arg(out.join(format!("{recipe}.dsn")))
        .output()?;
    let seconds = started.elapsed().as_secs_f64();
    let summary = String::from_utf8_lossy(&routed.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned();
    ensure!(
        routed.status.success(),
        "traxide did not route everything:\n{}",
        String::from_utf8_lossy(&routed.stdout)
    );
    let judgement = kicad.judge(
        root,
        &out,
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
        "clean in KiCad's DRC; routed in {seconds:.2} s ({})",
        summary.split(" → ").next().unwrap_or_default()
    ))
}
