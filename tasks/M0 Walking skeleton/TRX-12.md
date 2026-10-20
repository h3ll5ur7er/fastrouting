---
id: TRX-12
title: Route connections with multi-layer A* in the Detailed Routing Engine
type: feature
status: todo
priority: high
labels: [detailed-routing]
estimate: 5
layer: engine
---

The first detailed router: sequential, one connection at a time, enough for an end-to-end route.

- [ ] Two-pin decomposition by MST inside the engine for now (moves to Planning in [[TRX-24]])
- [ ] A* on an octilinear graph over a fine grid with via edges between layers; costs from the profile (length, via, bend)
- [ ] Exact clearance check against Layout Access before committing
- [ ] Unrouted connections reported, not dropped

**Depends on:** [[TRX-11]]
