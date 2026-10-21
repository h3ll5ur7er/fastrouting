---
id: TRX-2
title: Set up the Cargo workspace
type: chore
status: done
priority: high
labels: [infrastructure]
estimate: 1
layer: infrastructure
---

Create the workspace laid out in [[Development#repository-layout]], with the crates the walking skeleton needs, named `traxide-<component>`. Other crates arrive with the tasks that need them (YAGNI).

- [x] Workspace `Cargo.toml` with shared lints, edition 2024, pinned MSRV, release profile (`lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `strip`)
- [x] Each crate has a doc comment naming the volatility it hides
- [x] `traxide` binary in `clients/cli` that prints its version
- [x] `license = "MIT OR Apache-2.0"` in every crate

Done when `cargo build` and `cargo test` pass.

**Depends on:** [[TRX-1]]
