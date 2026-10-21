---
id: TRX-7
title: Read Specctra DSN in Board Access
type: feature
status: done
priority: high
labels: [board-access, dsn]
estimate: 5
layer: access
---

The `dsn` adapter of Board Access: `load_design("board.dsn")` returns a format-neutral `Design`.

- [x] Tokeniser and S-expression parser for DSN, tolerant of KiCad's and other tools' dialects
- [x] Structure: boundary, layers, rules, net classes, vias; placement and images (pins, padstacks); network; existing wiring; keepouts
- [x] Units and resolution converted to integer nm
- [x] Ground-truth test against KiCad 10's own pad placement (`tests/kicad_ground_truth.rs`); PCBench samples with [[TRX-15]]

**Depends on:** [[TRX-4]], [[TRX-5]]
