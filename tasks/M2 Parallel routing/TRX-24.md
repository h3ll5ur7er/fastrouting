---
id: TRX-24
title: Plan connections and batches in the Planning Engine
type: feature
status: in-progress
priority: high
labels: [planning]
estimate: 4
layer: engine
---

[[Architecture/Routing-pipeline#plan]].

- [ ] RSMT (FLUTE-style) per net, beyond the MST of [[TRX-46]]; ordered chains for key-matrix rows and columns
- [ ] Recognise switch–diode stubs and buses
- [ ] Escape patterns for QFN pads (RP2350: QFN-60 and QFN-80, 0.4 mm pitch)
- [x] Ordering: connections from hemmed-in pads first, then shortest first; ties by names, so the order a board lists its parts in changes nothing ([[Benchmarks#hemmed-in-pads-first-m2-trx-24]]). Batches come from detailed routing's territories ([[TRX-27]])

**Depends on:** [[TRX-13]]
