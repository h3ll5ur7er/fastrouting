---
id: TRX-27
title: Route batches in parallel with deterministic commits
type: feature
status: done
priority: high
labels: [detailed-routing, performance]
estimate: 5
layer: engine
---

[[Architecture/Parallelism-and-GPU#batches]].

- [x] Detailed routing takes batches of connections with apart territories from the queue; threads search, the engine commits in queue order
- [x] Conflicts move to the next batch: territories (window, copper reach and clearance) make them impossible, so a debug assertion checks it instead; a connection never overtakes one it could meet
- [x] Determinism tests: a scattered board on 1, 2 and 4 threads; the CLI's session of KiCad's matrix-3x4 on 1, 4 and 16 threads; keyboard-tkl and matrix-6x17 identical in nine runs on 1, 2 and 4 threads
- [x] Scaling report: 1 to 4 threads on the keyboard and matrix-6x17 ([[Benchmarks#parallel-batches-m2-trx-27]]); no machine with 24 cores yet

`traxide route --threads N` (all cores by default) and the profile's `batch_size`. Search memory is now tiled ([[crates/utilities/scratch/src/lib.rs]]), a quarter of what it was, so threads no longer cost memory they don't use.

Measured: no speed-up on the keyboards, whose time is in a few long connections that overlap everything; cheaper long searches ([[TRX-48]]) come next.

**Depends on:** [[TRX-23]], [[TRX-26]]
