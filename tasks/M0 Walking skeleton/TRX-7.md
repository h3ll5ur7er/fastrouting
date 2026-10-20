---
id: TRX-7
title: Read Specctra DSN in Board Access
type: feature
status: todo
priority: high
labels: [board-access, dsn]
estimate: 5
layer: access
---

The `dsn` adapter of Board Access: `load_design("board.dsn")` returns a format-neutral `Design`.

- [ ] Tokeniser and S-expression parser for DSN, tolerant of KiCad's and other tools' dialects
- [ ] Structure: boundary, layers, rules, net classes, vias; placement and images (pins, padstacks); network; existing wiring; keepouts
- [ ] Units and resolution converted to integer nm
- [ ] Golden tests on DSN files exported by KiCad 9 and 10 and on PCBench samples

**Depends on:** [[TRX-4]], [[TRX-5]]
