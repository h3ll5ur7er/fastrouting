---
id: TRX-48
title: Search free space instead of a fine grid
type: improvement
status: backlog
priority: medium
labels: [detailed-routing, performance]
estimate: 8
layer: engine
---

The detailed search walks a 0.1 mm grid: millions of states on a large board, most of them in open space where nothing needs deciding. Searching a graph of the places where something does change — corners of obstacles inflated by their clearance, the corridor's crossing points, a coarse grid in between — needs far fewer states for the same paths ([[Research/Algorithms#classical-baselines]]).

- [ ] Implicit octilinear graph from corners of inflated obstacles plus a fallback grid
- [ ] Same results as the grid search on the end-to-end boards, measured in [[Benchmarks]]
- [ ] Decide after [[TRX-26]]: corridors may shrink the grid search enough

**Depends on:** [[TRX-23]]
