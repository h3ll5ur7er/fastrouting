---
id: TRX-6
title: Build the Diagnostics, Events and Work pool utilities
type: feature
status: todo
priority: medium
labels: [infrastructure]
estimate: 2
layer: utility
---

- [ ] Diagnostics: `tracing` setup, spans per stage, counters and timings that end up in the `Report`
- [ ] Events: progress events, cancellation tokens, an in-process job queue between managers ([[Architecture/Use-cases#uc6-run-the-benchmark-corpus]])
- [ ] Work pool: rayon pool with a configurable thread count and deterministic ordered reductions

**Depends on:** [[TRX-2]]
