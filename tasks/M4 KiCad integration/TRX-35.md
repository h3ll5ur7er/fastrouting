---
id: TRX-35
title: Release static binaries for six targets
type: chore
status: todo
priority: high
labels: [release, ci]
estimate: 2
layer: infrastructure
---

[[Research/Tech-stack#distribution]].

- [ ] cargo-dist workflow: Linux glibc 2.28 (GPU) and musl (CPU-only), Windows MSVC `crt-static`, macOS universal, x86-64 and ARM64
- [ ] macOS signing and notarisation; optional Windows signing
- [ ] Job that assembles the KiCad PCM packages
- [ ] `cargo deny` licence check in the release: only permissive licences in the binary

**Depends on:** [[TRX-3]], [[TRX-14]]
