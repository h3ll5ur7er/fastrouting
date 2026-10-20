---
id: TRX-12
title: Route connections with multi-layer A* in the Detailed Routing Engine
type: feature
status: done
priority: high
labels: [detailed-routing]
estimate: 5
layer: engine
---

The first detailed router: sequential, one connection at a time, enough for an end-to-end route.

- [x] Routes the connections of a `Plan` in order
- [x] A* on an octilinear graph over a fine grid with via edges between layers; costs from the profile (length, via, bend)
- [x] Exact clearance check against Layout Access before committing
- [x] Unrouted connections reported, not dropped

**Depends on:** [[TRX-11]], [[TRX-46]]
