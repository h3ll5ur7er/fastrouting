//! The real-world test: KiCad builds each board and exports it, traxide routes
//! it, KiCad imports the session and KiCad's own DRC judges the result.
//!
//! KiCad runs in the official Docker image (`TRAXIDE_KICAD_IMAGE`, by default
//! kicad/kicad:10.0), or, with `--local`, as installed: `TRAXIDE_KICAD_PYTHON`
//! (a Python that can `import pcbnew`) and `TRAXIDE_KICAD_CLI`.

use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

const DEFAULT_RECIPES: &[&str] = &["placements", "crossing", "matrix-3x4"];

enum Kicad {
    Docker { image: String },
    Local { python: String, cli: String },
}

impl Kicad {
    /// Runs `script` (a shell command line) with the e2e scripts as working
    /// directory; `{out}` in it stands for `out`.
    fn run(&self, root: &Path, out: &Path, script: &str) -> Result<String> {
        let mut command = match self {
            Self::Docker { image } => {
                let mut command = Command::new("docker");
                command.args(["run", "--rm", "-e", "HOME=/tmp", "-w", "/work/e2e/kicad"]);
                if let Some(user) = current_user() {
                    command.args(["--user", &user]);
                }
                command
                    .arg("-v")
                    .arg(format!("{}:/work:ro", root.display()))
                    .arg("-v")
                    .arg(format!("{}:/out", out.display()))
                    .args([image.as_str(), "sh", "-c", &script.replace("{out}", "/out")]);
                command
            }
            Self::Local { python, cli } => {
                let script = script
                    .replace("{out}", &out.display().to_string())
                    .replace("python3 ", &format!("{python} "))
                    .replace("kicad-cli ", &format!("{cli} "));
                let mut command = Command::new("sh");
                command
                    .args(["-c", &script])
                    .current_dir(root.join("e2e/kicad"));
                command
            }
        };
        let output = command.output().context("running KiCad")?;
        let text = String::from_utf8_lossy(&output.stdout).into_owned()
            + &String::from_utf8_lossy(&output.stderr);
        ensure!(output.status.success(), "KiCad failed: {script}\n{text}");
        Ok(text)
    }
}

/// So that files KiCad writes into the mounted folder belong to the caller.
fn current_user() -> Option<String> {
    let id = |flag| {
        Command::new("id")
            .arg(flag)
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
    };
    Some(format!("{}:{}", id("-u")?, id("-g")?))
}

#[derive(Debug, Deserialize)]
struct DrcReport {
    violations: Vec<Finding>,
    unconnected_items: Vec<Finding>,
}

#[derive(Debug, Deserialize)]
struct Finding {
    description: String,
    #[serde(default)]
    items: Vec<FindingItem>,
}

#[derive(Debug, Deserialize)]
struct FindingItem {
    description: String,
}

pub fn run(args: &[String]) -> Result<()> {
    let local = args.iter().any(|a| a == "--local");
    let recipes: Vec<&str> = args
        .iter()
        .filter(|a| !a.starts_with("--"))
        .map(String::as_str)
        .collect();
    let recipes = if recipes.is_empty() {
        DEFAULT_RECIPES.to_vec()
    } else {
        recipes
    };
    let kicad = if local {
        Kicad::Local {
            python: std::env::var("TRAXIDE_KICAD_PYTHON").unwrap_or_else(|_| "python3".into()),
            cli: std::env::var("TRAXIDE_KICAD_CLI").unwrap_or_else(|_| "kicad-cli".into()),
        }
    } else {
        Kicad::Docker {
            image: std::env::var("TRAXIDE_KICAD_IMAGE")
                .unwrap_or_else(|_| "kicad/kicad:10.0".into()),
        }
    };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()?;
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let built = Command::new(&cargo)
        .args(["build", "--release", "--quiet", "-p", "traxide-cli"])
        .current_dir(&root)
        .status()?;
    ensure!(built.success(), "building traxide failed");
    let traxide = root.join("target/release/traxide");
    let mut failures = Vec::new();
    for recipe in recipes {
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
    kicad.run(
        root,
        &out,
        &format!("python3 roundtrip.py export {recipe} {{out}}"),
    )?;
    let started = Instant::now();
    let routed = Command::new(traxide)
        .arg("route")
        .arg("--quiet")
        .arg(out.join(format!("{recipe}.dsn")))
        .output()?;
    let seconds = started.elapsed().as_secs_f64();
    let summary = String::from_utf8_lossy(&routed.stdout)
        .trim()
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned();
    ensure!(
        routed.status.success(),
        "traxide did not route everything:\n{}",
        String::from_utf8_lossy(&routed.stdout)
    );
    kicad.run(
        root,
        &out,
        &format!(
            "python3 roundtrip.py import {{out}}/{recipe}.kicad_pcb {{out}}/{recipe}.ses && \
             kicad-cli pcb drc --format json --severity-error -o {{out}}/drc.json {{out}}/{recipe}.routed.kicad_pcb"
        ),
    )?;
    let report: DrcReport = serde_json::from_str(&std::fs::read_to_string(out.join("drc.json"))?)?;
    let findings: Vec<String> = report
        .violations
        .iter()
        .chain(&report.unconnected_items)
        .map(|f| {
            format!(
                "  {}: {}",
                f.description,
                f.items
                    .iter()
                    .map(|i| i.description.as_str())
                    .collect::<Vec<_>>()
                    .join(" / ")
            )
        })
        .collect();
    ensure!(
        findings.is_empty(),
        "KiCad's DRC found {} violations and {} unconnected items:\n{}",
        report.violations.len(),
        report.unconnected_items.len(),
        findings.join("\n")
    );
    Ok(format!(
        "clean in KiCad's DRC; routed in {seconds:.2} s ({})",
        summary.split(" → ").next().unwrap_or_default()
    ))
}
