---
id: TRX-70
title: Write the fixed wiring a DSN came with as protected, not as new routes
type: bug
status: todo
priority: high
labels: [dsn, ses]
layer: access
---

From the TokenBoard report ([[Research/TokenBoard-report]]): the DSN's `wiring` held 655 wires and 439 vias marked `(type fix)`; traxide's session wrote every one of them again as plain routes. After `ImportSpecctraSES` KiCad keeps its locked pre-routing and adds the copies: 199 `holes_co_located`. Freerouting writes the wiring it was given with `(type protect)` (the report's `kb.freerouting-again.ses`).

- [ ] A failing test: a design's fixed wiring comes back in the session marked `(type protect)`, the new routes unmarked
- [ ] Decide from how KiCad imports a session: with `(type protect)` as Freerouting does, or left out
- [ ] The report's board: no co-located holes after the import
