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
- [ ] Run Freerouting 2.5 on each, record time, memory, completion, vias, length and DRC; this is the baseline
- [ ] Record the KiCad DRC result of each baseline with `kicad-cli pcb drc`

**Depends on:** [[TRX-1]]
