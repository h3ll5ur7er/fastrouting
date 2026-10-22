---
id: TRX-71
title: Keep same-net vias the hole-to-hole distance apart
type: bug
status: review
assignee: claude
priority: high
labels: [detailed-routing, rules, verification]
layer: engine
---

From the TokenBoard report ([[Research/TokenBoard-report]]): KiCad reports 11 `hole_to_hole` errors, all between same-net vias that traxide placed: some overlap (gap 0), five are exact duplicates within the session (same net, same position), others are 0.1–0.23 mm apart where KiCad's default minimum is 0.25 mm. A DSN does not carry KiCad's hole-to-hole minimum; a `.kicad_pro` does (`min_hole_to_hole`).

- [x] A failing test: a route that would place a via within the hole-to-hole distance of another via of its net, or on it, does not
- [x] Find where duplicates come from (two connections of one net each dropping a via at the same node, or a route and an existing via)
- [x] The hole-to-hole distance from the board's rules, 0.25 mm by default; verification reports it
- [x] The report's board: no `hole_to_hole` errors. Measured by a script apart from traxide: no two holes closer than 0.25 mm and no duplicates, where the report's session had 11 and 5

**What it was.** The layout checked a via only against the copper of other nets, so a via could sit anywhere on copper of its own net, overlapping or beside another via of it. Two connections of a net that each pass through one node each placed a via there, and the session wrote both.

**What changed.**
- `DesignRules.hole_to_hole`: from `min_hole_to_hole` of the `.kicad_pro`, 0.25 mm when it has none; a DSN carries none, so 0.25 mm. The compiled rules carry it as `ConstraintSet::hole_to_hole`.
- The layout indexes the holes of existing and routed vias. A via is obstructed by any hole closer than the distance, whatever its net, except a via of its own net and kind on the same spot, which is the same hole.
- The routing manager leaves out a via that repeats one already there or placed before (`Routes::without_repeated_vias`).
- Verification reports `Violation::HoleToHole` between vias, which the DRC report writes as KiCad's `hole_to_hole`. Drilled pads are not judged: the design does not carry pad drills.
- A DSN carries no via drill, but KiCad names the via padstacks it exports after their diameter and drill (`Via[0-3]_600:300_um`), so the reader takes the drill from the name of a KiCad design's via; other tools' vias have no known drill and keep no hole-to-hole distance.
- A search changes layers through a via of its net the board came with, where its copper covers a grid node on both layers, instead of placing a via of its own beside it. Locked vias seldom sit on the grid, so without this the rule kept new vias away from them: TokenBoard's KiCad board fell from 240 nets in 54 s to 237 against the clock, its RP2350 fan-out crowded by +3V3 routes that could no longer go down beside its locked vias. With it, 241 of 241 nets in 37 s and 523 vias instead of 553. The layer change costs as much as a via: free, paths crowded through the board's vias, and Issue110-RelayModule routed 63 of 64 nets in 91 s instead of all in 21 s. Vias other connections routed may be ripped up, so a search does not go through them.
- A search keeps a via it would place the distance from the vias its own path placed, walking its path back as far as one could be: the layout holds only other connections' routes, and a hop over an obstacle on one layer put two vias of one path 0.3 mm apart on Issue283.
