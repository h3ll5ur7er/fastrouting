---
id: TRX-45
title: Test end to end through KiCad 10
type: feature
status: review
priority: high
labels: [e2e, kicad, ci]
estimate: 3
layer: infrastructure
---

KiCad, not our code, judges whether a routed board is right ([[Development#testing]]).

- [x] Board recipes generated with KiCad's Python API: placements on both sides, a crossing, key matrices of 3×4 and 6×17 (4-layer recipes come with the keyboard of [[TRX-15]])
- [x] Per recipe: export DSN, run `traxide route`, import the SES with `ImportSpecctraSES(board, file)`, run KiCad's DRC; fail on any violation or unconnected item
- [x] `cargo xtask e2e` runs the harness in the official `kicad/kicad:10.0` Docker image, or with a local KiCad 10 when `--local` is given
- [x] CI job on Linux; also against `kicad/kicad:9.0`
- [x] Never against KiCad 7 or 8: they are not the versions users route with

**Depends on:** [[TRX-14]]
