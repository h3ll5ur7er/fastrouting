---
id: FR-3
title: Enforce the layer rules and run CI on all platforms
type: chore
status: todo
priority: high
labels: [infrastructure, ci]
estimate: 1.5
layer: infrastructure
---

Make the iDesign call rules a build failure, as described in [[Development#layer-rules]].

- [ ] `xtask` crate with `cargo xtask layers`: read `cargo metadata`, fail on forbidden dependencies (engine → engine, access → access, client → engine, anything upward)
- [ ] GitHub Actions: fmt, clippy `-D warnings`, tests, layer check on Linux, macOS and Windows, x86-64 and ARM64 runners
- [ ] A deliberately forbidden dependency in a test fixture proves the check fails

**Depends on:** [[FR-2]]
