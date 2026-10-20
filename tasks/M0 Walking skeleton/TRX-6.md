---
id: TRX-6
title: Build the Events utility
type: feature
status: done
priority: medium
labels: [infrastructure]
estimate: 2
layer: utility
---

- [x] Progress events from managers to clients
- [x] Cancellation tokens

Not yet (YAGNI): the job queue between managers comes with the benchmark ([[TRX-22]]), the work pool with parallel routing ([[TRX-27]]). Diagnostics use `tracing` directly; a utility of their own only if setup grows beyond the CLI.

**Depends on:** [[TRX-2]]
