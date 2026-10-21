---
id: TRX-5
title: Build the Geometry utility
type: feature
status: done
priority: high
labels: [geometry]
estimate: 5
layer: utility
---

Integer-nanometre geometry for every layer ([[Architecture/Contracts#units-and-geometry]]).

- [x] `Point`, `Rect`, `Segment`, `Polygon`, `Shape` (capsules and polygons) and `Transform` on `i32` nm, exact `i128` predicates, boards limited to ±1 m
- [x] Orientation and intersection predicates; segment–segment, segment–arc and point–polygon distances
- Octilinear helpers, arcs, offsetting and `criterion` benchmarks: when a task needs them (YAGNI)
- [x] Property tests (`proptest`) for the distance functions

**Depends on:** [[TRX-2]]
