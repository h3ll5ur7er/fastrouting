---
id: TRX-31
title: Talk to a running KiCad through the IPC API in Board Access
type: feature
status: todo
priority: high
labels: [board-access, kicad]
estimate: 5
layer: access
---

The `kicad-ipc` adapter.

- [ ] Read board, nets, stackup, pad polygons through IPC; rules from the project files on KiCad 10, over IPC on KiCad 11
- [ ] Store routes as one commit (one undo), then refill zones
- [ ] Clear error when the API is disabled, with the steps to enable it

**Depends on:** [[TRX-30]], [[TRX-17]]
