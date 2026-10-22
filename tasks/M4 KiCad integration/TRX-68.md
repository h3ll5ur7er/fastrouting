---
id: TRX-68
title: Read the windows of DSN planes, keepouts and boundaries
type: bug
status: todo
priority: high
labels: [dsn, kicad]
layer: access
---

From the TokenBoard report ([[Research/TokenBoard-report]]): `traxide route kb.dsn` stops with `line 55: unsupported shape window`. KiCad 10's `ExportSpecctraDSN` writes a `(window (polygon …))` for every hole of a zone: here the VLED plane on In2.Cu has one where the +3V3 island sits. Any board with a split plane hits this.

- [ ] A failing test: a plane with a window reads, and its window is a hole of it
- [ ] Planes, keepouts and the boundary take windows as holes; a plane's fill and its joins ([[TRX-58]], [[TRX-67]]) leave them out
- [ ] The report's `kb.dsn` reads and routes
