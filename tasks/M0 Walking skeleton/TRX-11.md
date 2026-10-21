---
id: TRX-11
title: Store the routing state in Layout Access
type: feature
status: done
priority: high
labels: [layout, performance]
estimate: 5
layer: access
---

The in-memory routing database ([[Architecture/Volatilities]] V11).

- [x] `build_layout(design, constraints)`: pads, keepouts, outline and existing copper as obstacles inflated per net class and layer
- [x] Uniform grid of buckets per layer, SoA storage; `obstacles_near`, `clearance_violations`
- Transactions and rip-up: with [[TRX-27]] and [[TRX-23]], when something uses them
- Property test of concurrent transactions: with [[TRX-27]]

**Depends on:** [[TRX-4]], [[TRX-5]], [[TRX-10]]
