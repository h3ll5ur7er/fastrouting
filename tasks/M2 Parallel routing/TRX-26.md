---
id: TRX-26
title: Route globally with negotiated congestion
type: feature
status: todo
priority: high
labels: [global-routing]
estimate: 6
layer: engine
---

[[Architecture/Routing-pipeline#global-routing]].

- [ ] Coarse 3D grid with capacities from rasterised obstacles
- [ ] Pattern routing (L, Z, 3-bend, layer DP), then PathFinder negotiation on overflow
- [ ] Track assignment of border crossings; corridors as output
- [ ] Routing Manager runs Plan → Global → Detailed

**Depends on:** [[TRX-24]], [[TRX-25]]
