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

- [x] Import the real boards of Freerouting's repository: not a "PCBench" of 1,157 boards, as our research had it, but 76 DSN files from its bug reports in `fixtures/`. `cargo xtask corpus` downloads them at run time ([[corpus/freerouting-fixtures.txt]]), routes each and reports a table ([[Benchmarks]])
- [x] Analyse why traxide leaves corpus boards incomplete: [[Corpus-analysis]], with the fixes as [[TRX-54]] to [[TRX-60]]
- [ ] PCBWorld-Bench boards, respecting their licences
- [x] Add our own boards: `keyboard-tkl`, a 4-layer RP2350 ISO TKL (88 keys, NKRO, USB-C, flash, crystal), and the key matrices; a macropad when needed
- [x] Run Freerouting 2.5 on our boards and record time, completion, vias, length and KiCad DRC: the Benchmark workflow (`cargo xtask bench`), results in [[Benchmarks#traxide-against-freerouting-250]]; memory not yet measured
- [x] Record the KiCad DRC result of each baseline with `kicad-cli pcb drc`

**Depends on:** [[TRX-1]]
