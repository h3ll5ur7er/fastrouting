---
id: TRX-26
title: Route globally with negotiated congestion
type: feature
status: done
priority: high
labels: [global-routing]
estimate: 6
layer: engine
---

[[Architecture/Routing-pipeline#global-routing]].

- [x] Coarse 3D grid with lane capacities from the bounding boxes of pads and keepouts
- [x] PathFinder negotiation (A* per connection with present and history costs, 8 rounds); pattern routing not needed at this size
- [x] Corridors as output: each connection's cells widened by one
- [x] Routing Manager runs Plan → Global → Detailed when the profile asks for it

Measured on keyboard-tkl and matrix-6x17, corridors made detailed routing slower, so `global_routing` is off by default; see [[Benchmarks#global-routing-corridors-measured-and-off-m2-trx-26]] for the numbers and the next ideas. Track assignment and Compute Access ([[TRX-25]]) wait until corridors pay off.

**Depends on:** [[TRX-24]], [[TRX-25]]
