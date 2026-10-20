---
id: TRX-47
title: Keep straight connections octilinear
type: improvement
status: todo
priority: medium
labels: [detailed-routing]
estimate: 1
layer: engine
---

The Detailed Routing Engine's shortcut — a straight track when the two pad centres see each other — produces tracks at any angle (see the picture in [[Benchmarks#walking-skeleton-m0]]). KiCad accepts them, but the architecture promises octilinear tracks by default ([[Architecture/Contracts#units-and-geometry]]), and designers expect them.

- [ ] Replace the any-angle shortcut by the best of the two octilinear doglegs (straight, then 45°, or the other way round) that fits
- [ ] Keep the any-angle line only when a profile asks for it, should anyone want it
- [ ] Test: every track segment the router produces is horizontal, vertical or diagonal, except the stubs that leave off-grid pads

**Depends on:** [[TRX-12]]
