---
id: TRX-50
title: Negotiate with history costs where routes collide
type: improvement
status: done
priority: high
labels: [detailed-routing, performance]
estimate: 1
layer: engine
---

After [[TRX-24]]'s ordering, the keyboard's MCU connections still ripped each other up 96 times, at the same few millimetres around the QFN's pins. Rip-up prices only protect the connection ripped up; PathFinder also remembers *where* collisions happen.

- [x] A history grid of 0.5 mm cells per layer: every segment or via of a committed route that displaced others makes its cells `history_cost` dearer for every later search
- [x] Searches pay the history of the cells they cross, folded into their cached edge and via costs
- [x] `history_cost` in profiles, 2 mm (a via) by default, 0 turns it off ([[Profiles]])
- [x] Measured: keyboard-tkl 6.6 s → 3.3 s, 117 → 107 vias, clean in KiCad 10's DRC ([[Benchmarks#history-costs-m2-trx-50]])
