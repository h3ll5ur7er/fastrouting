---
id: TRX-46
title: Decompose nets into connections in the Planning Engine
type: feature
status: done
priority: high
labels: [planning]
estimate: 1
layer: engine
---

The first slice of the Planning Engine ([[Architecture/Routing-pipeline#plan]]), so the Detailed Routing Engine receives connections instead of deciding them itself.

- [x] Minimum spanning tree over each net's pads (Manhattan distance), deterministic tie-breaks
- [x] Connections ordered shortest first

Ordering heuristics, Steiner trees and batches follow in [[TRX-24]].

**Depends on:** [[TRX-4]]
