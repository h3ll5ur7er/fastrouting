---
id: TRX-14
title: Route from the command line
type: feature
status: done
priority: high
labels: [cli]
estimate: 1.5
layer: client
---

`traxide route board.dsn -o board.ses [--profile fast] [--threads N]`.

- [x] `clap` command tree with `route` (later `optimize`, `check`, `bench`)
- [x] Progress bar from events; summary at the end; exit codes for unrouted nets
- [x] Walking skeleton done: a simple 2-layer board routes end to end and imports into KiCad

**Depends on:** [[TRX-13]]
