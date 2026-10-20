---
id: TRX-5
title: Build the Geometry utility
type: feature
status: todo
priority: high
labels: [geometry]
estimate: 5
layer: utility
---

Integer-nanometre geometry for every layer ([[Architecture/Contracts#units-and-geometry]]).

- [ ] `Point`, `Vector`, `Rect`, `Segment`, `Arc`, `Polygon` on `i32` nm, exact `i64`/`i128` products, boards limited to ±1 m
- [ ] Orientation and intersection predicates; segment–segment, segment–arc and point–polygon distances
- [ ] Octilinear helpers: snapping, 45° corners, octagons for pads and vias
- [ ] Polygon offsetting and booleans through `i_overlay` / `clipper2-rust`
- [ ] Property tests (`proptest`) and benchmarks (`criterion`) for the distance functions

**Depends on:** [[TRX-2]]
