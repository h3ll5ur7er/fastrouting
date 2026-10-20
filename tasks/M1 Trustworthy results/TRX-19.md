---
id: TRX-19
title: Check designs independently in the Verification Engine
type: feature
status: in-progress
priority: high
labels: [verification, drc]
estimate: 5
layer: engine
---

The referee ([[Architecture/Volatilities]] V9): never shares clearance code with the router.

- [x] Clearance between all copper of different nets per layer, outline and keepouts (sweep along x, independent of Layout Access)
- [ ] Widths and via rules
- [x] Unconnected pins
- [ ] Dangling copper
- [ ] Metrics: completion, total length, vias, per-net length
- [ ] Parallel by tile
- [x] Tested on boards with seeded violations

**Depends on:** [[TRX-10]], [[TRX-11]]
