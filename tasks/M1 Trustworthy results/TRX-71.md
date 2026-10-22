---
id: TRX-71
title: Keep same-net vias the hole-to-hole distance apart
type: bug
status: todo
priority: high
labels: [detailed-routing, rules, verification]
layer: engine
---

From the TokenBoard report ([[Research/TokenBoard-report]]): KiCad reports 11 `hole_to_hole` errors, all between same-net vias that traxide placed: some overlap (gap 0), five are exact duplicates within the session (same net, same position), others are 0.1–0.23 mm apart where KiCad's default minimum is 0.25 mm. A DSN does not carry KiCad's hole-to-hole minimum; a `.kicad_pro` does (`min_hole_to_hole`).

- [ ] A failing test: a route that would place a via within the hole-to-hole distance of another via of its net, or on it, does not
- [ ] Find where duplicates come from (two connections of one net each dropping a via at the same node, or a route and an existing via)
- [ ] The hole-to-hole distance from the board's rules, 0.25 mm by default; verification reports it
- [ ] The report's board: no `hole_to_hole` errors
