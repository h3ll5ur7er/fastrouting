---
id: TRX-45
title: Test end to end through KiCad 10
type: feature
status: todo
priority: high
labels: [e2e, kicad, ci]
estimate: 3
layer: infrastructure
---

KiCad, not our code, judges whether a routed board is right ([[Development#testing]]).

- [ ] Board recipes generated with KiCad's Python API: two pads, a few nets on 2 layers, a small key matrix with diodes on 4 layers
- [ ] Per recipe: export DSN, run `traxide route`, import the SES with `ImportSpecctraSES(board, file)`, run KiCad's DRC; fail on any violation or unconnected item
- [ ] `cargo xtask e2e` runs the harness in the official `kicad/kicad:10.0` Docker image, or with a local KiCad 10 when `--local` is given
- [ ] CI job on Linux; also against `kicad/kicad:9.0`
- [ ] Never against KiCad 7 or 8: they are not the versions users route with

**Depends on:** [[TRX-14]]
