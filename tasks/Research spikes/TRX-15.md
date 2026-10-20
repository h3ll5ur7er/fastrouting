---
id: TRX-15
title: Assemble the benchmark corpus and Freerouting baselines
type: research
status: in-progress
priority: high
labels: [benchmark]
estimate: 3
---

The acceptance tests ([[Research/Prior-art#benchmark-corpora]]).

- [ ] Import PCBench (Freerouting repository) and PCBWorld-Bench boards, respecting their licences (Git LFS or a download script)
- [x] Add our own boards: `keyboard-tkl`, a 4-layer RP2350 ISO TKL (88 keys, NKRO, USB-C, flash, crystal), and the key matrices; a macropad when needed
- [x] Run Freerouting 2.5 on our boards and record time, completion, vias, length and KiCad DRC: the Benchmark workflow (`cargo xtask bench`), results in [[Benchmarks#traxide-against-freerouting-250]]; memory not yet measured
- [x] Record the KiCad DRC result of each baseline with `kicad-cli pcb drc`

**Depends on:** [[TRX-1]]
