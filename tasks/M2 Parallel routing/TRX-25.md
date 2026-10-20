---
id: TRX-25
title: Run data-parallel kernels on the CPU in Compute Access
type: feature
status: todo
priority: high
labels: [compute, performance]
estimate: 3
layer: access
---

The CPU backend of Compute Access, and the reference for every GPU kernel later.

- [ ] `pattern_route`, `update_congestion`, `expand_wavefronts`, `rasterise_obstacles` on rayon + `fearless_simd`/`pulp`, runtime dispatch per CPU
- [ ] Deterministic results independent of thread count

**Depends on:** [[TRX-4]], [[TRX-6]]
