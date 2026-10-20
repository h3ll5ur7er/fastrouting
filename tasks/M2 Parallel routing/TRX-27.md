---
id: TRX-27
title: Route batches in parallel with deterministic commits
type: feature
status: todo
priority: high
labels: [detailed-routing, performance]
estimate: 5
layer: engine
---

[[Architecture/Parallelism-and-GPU#batches-and-transactions]].

- [ ] Detailed routing colours corridors into batches; workers stage, the engine commits in connection order
- [ ] Conflicts move to the next batch
- [ ] Determinism test: every corpus board with 1, 4 and all threads gives byte-identical SES
- [ ] Scaling report: speed-up from 1 to 24 threads on the keyboard and the largest corpus boards

**Depends on:** [[TRX-23]], [[TRX-26]]
