//! Enforces the iDesign call rules of docs/Development.md: a crate may only depend
//! on crates of the layers its own layer may call.

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layer {
    Client,
    Manager,
    Engine,
    Access,
    Utility,
    Contract,
}

impl Layer {
    fn of(manifest_path: &str) -> Option<Self> {
        let path = manifest_path.replace('\\', "/");
        [
            ("/crates/clients/", Self::Client),
            ("/crates/managers/", Self::Manager),
            ("/crates/engines/", Self::Engine),
            ("/crates/access/", Self::Access),
            ("/crates/utilities/", Self::Utility),
            ("/crates/contracts/", Self::Contract),
        ]
        .into_iter()
        .find_map(|(folder, layer)| path.contains(folder).then_some(layer))
    }

    fn may_use(self, other: Self) -> bool {
        use Layer::*;
        match self {
            Client => matches!(other, Manager | Contract | Utility),
            Manager => matches!(other, Engine | Access | Contract | Utility),
            // Engines reach resource access only through traits in the contracts.
            Engine | Access => matches!(other, Contract | Utility),
            Utility | Contract => matches!(other, Utility),
        }
    }
}

#[derive(Debug, Deserialize)]
struct Metadata {
    packages: Vec<Package>,
}

#[derive(Debug, Deserialize)]
struct Package {
    name: String,
    manifest_path: String,
    dependencies: Vec<Dependency>,
}

#[derive(Debug, Deserialize)]
struct Dependency {
    name: String,
    /// `None` for normal dependencies; tests and build scripts may use anything.
    kind: Option<String>,
}

pub fn check() -> Result<()> {
    let output = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .context("running cargo metadata")?;
    if !output.status.success() {
        bail!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let metadata: Metadata =
        serde_json::from_slice(&output.stdout).context("reading cargo metadata")?;
    let violations = violations(&metadata.packages);
    if violations.is_empty() {
        println!(
            "layers: {} crates follow the call rules",
            metadata.packages.len()
        );
        Ok(())
    } else {
        bail!("layer rules broken:\n  {}", violations.join("\n  "))
    }
}

fn violations(packages: &[Package]) -> Vec<String> {
    let layer_of = |name: &str| {
        packages
            .iter()
            .find(|p| p.name == name)
            .and_then(|p| Layer::of(&p.manifest_path))
    };
    let mut found = Vec::new();
    for package in packages {
        let Some(layer) = Layer::of(&package.manifest_path) else {
            continue;
        };
        for dependency in package.dependencies.iter().filter(|d| d.kind.is_none()) {
            let Some(used) = layer_of(&dependency.name) else {
                continue;
            };
            let same_layer_peer =
                used == layer && matches!(layer, Layer::Engine | Layer::Access | Layer::Manager);
            if !layer.may_use(used) || same_layer_peer {
                found.push(format!(
                    "{} ({layer:?}) must not use {} ({used:?})",
                    package.name, dependency.name
                ));
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(name: &str, folder: &str, uses: &[&str]) -> Package {
        Package {
            name: name.into(),
            manifest_path: format!("/repo/crates/{folder}/{name}/Cargo.toml"),
            dependencies: uses
                .iter()
                .map(|&n| Dependency {
                    name: n.into(),
                    kind: None,
                })
                .collect(),
        }
    }

    #[test]
    fn a_manager_may_use_engines_and_access() {
        let packages = [
            package("m", "managers", &["e", "a"]),
            package("e", "engines", &[]),
            package("a", "access", &[]),
        ];
        assert!(violations(&packages).is_empty());
    }

    #[test]
    fn an_engine_must_not_use_another_engine() {
        let packages = [
            package("e1", "engines", &["e2"]),
            package("e2", "engines", &[]),
        ];
        assert_eq!(
            violations(&packages),
            ["e1 (Engine) must not use e2 (Engine)"]
        );
    }

    #[test]
    fn an_engine_must_not_use_resource_access_directly() {
        let packages = [package("e", "engines", &["a"]), package("a", "access", &[])];
        assert_eq!(violations(&packages).len(), 1);
    }

    #[test]
    fn a_client_must_not_skip_the_managers() {
        let packages = [
            package("c", "clients", &["e"]),
            package("e", "engines", &[]),
        ];
        assert_eq!(violations(&packages).len(), 1);
    }

    #[test]
    fn nothing_calls_up() {
        let packages = [
            package("a", "access", &["m"]),
            package("m", "managers", &[]),
        ];
        assert_eq!(violations(&packages).len(), 1);
    }

    #[test]
    fn tests_may_use_any_crate() {
        let mut client = package("c", "clients", &[]);
        client.dependencies.push(Dependency {
            name: "e".into(),
            kind: Some("dev".into()),
        });
        let packages = [client, package("e", "engines", &[])];
        assert!(violations(&packages).is_empty());
    }
}
