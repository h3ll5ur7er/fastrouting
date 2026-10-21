---
id: TRX-20
title: Check a board from the command line
type: feature
status: in-progress
priority: high
labels: [manager, cli]
estimate: 2
layer: manager
---

UC5 of [[Architecture/Use-cases#uc5-check-a-board]] in the Inspection Manager, and `traxide check board.dsn [board.ses]`.

- [x] Load design and routes, compile rules, verify: `traxide check BOARD.dsn [SESSION.ses]` in a new Inspection Manager crate; exits 0 when complete and clean, 2 when incomplete, 3 when a rule is broken
- [x] Text and JSON (`--json`) reports: nets complete, pads unconnected, violations by kind, tracks, vias, track length
- [ ] KiCad-JSON report ([[TRX-51]]); recording reports ([[TRX-21]])

**Depends on:** [[TRX-19]], [[TRX-21]]
