---
id: TRX-49
title: Expand each search state once
type: improvement
status: done
priority: high
labels: [detailed-routing, performance]
estimate: 0.5
layer: engine
---

Found while tracing [[TRX-27]]: with the weighted estimate, the keyboard's longest search expanded 3.7 million times but only 590 000 distinct states, each more than six times on average, because a bold search reaches states first by worse ways round. Weighted A* that never reopens a state keeps its bound, a path within the weight of the cheapest.

- [x] Failing test: a weighted search around a cup expands no state twice
- [x] A state's best cost becomes `CLOSED` once it is expanded
- [x] Measured on the end-to-end boards, all clean in KiCad 10's DRC ([[Benchmarks#each-state-expanded-once-m2-trx-49]])

The keyboard gains less than the matrices: its long MCU connections now take turns ripping each other up, 5 to 8 searches each, for the QFN's escape room ([[TRX-24]]).
