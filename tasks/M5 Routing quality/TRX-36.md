---
id: TRX-36
title: Optimise routes in the Optimization Engine
type: feature
status: backlog
priority: medium
labels: [optimization]
estimate: 6
layer: engine
---

[[Architecture/Routing-pipeline#optimise]]; UC4 *optimise an existing routing* in the Routing Manager.

- [ ] Pull-tight in each trace's channel; 45° mitering; stub removal
- [ ] Via-pair removal and layer reassignment
- [ ] Region-parallel on copy-on-write snapshots; a move is kept only if it improves and adds no violation

**Depends on:** [[TRX-27]], [[TRX-19]]
