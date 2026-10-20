---
id: TRX-2
title: Set up the Cargo workspace with one crate per component
type: chore
status: todo
priority: high
labels: [infrastructure]
estimate: 1
layer: infrastructure
---

Create the workspace laid out in [[Development#repository-layout]]: empty crates for every client, manager, engine, resource access and utility, named `traxide-<component>`, with the dependencies the layer rules allow.

- [ ] Workspace `Cargo.toml` with shared lints, edition 2024, pinned MSRV, release profile (`lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `strip`)
- [ ] One crate per component, each with a crate doc comment naming the volatility it hides
- [ ] `traxide` binary in `clients/cli` that prints its version
- [ ] `license = "MIT OR Apache-2.0"` in every crate

Done when `cargo build` and `cargo test` pass on an empty workspace.

**Depends on:** [[TRX-1]]
