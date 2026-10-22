---
id: TRX-70
title: Write the fixed wiring a DSN came with as protected, not as new routes
type: bug
status: in-progress
assignee: claude
priority: high
labels: [dsn, ses]
layer: access
---

From the TokenBoard report ([[Research/TokenBoard-report]]): the DSN's `wiring` held 655 wires and 439 vias marked `(type fix)`; traxide's session wrote every one of them again as plain routes. After `ImportSpecctraSES` KiCad keeps its locked pre-routing and adds the copies: 199 `holes_co_located`. Freerouting writes the wiring it was given with `(type protect)` (the report's `kb.freerouting-again.ses`).

- [x] A failing test: a session leaves out the wiring that came fixed or protected, and keeps the board's other wiring
- [x] Decided from how KiCad 10 imports a session, on the `crossing` board with a locked track and via exported as `(type fix)`: written plainly or as `(type protect)`, KiCad keeps its locked copper and adds the session's copies (6 tracks, 4 vias, 1 co-located); left out, it keeps its own (5 tracks, 3 vias, none co-located). So a design keeps its locked copper apart (`Design::locked`, from `(type fix)` and `(type protect)` wiring), and a session leaves it out. Unlocked wiring is still written: the import replaces it
- [ ] The report's board: no co-located holes after the import
