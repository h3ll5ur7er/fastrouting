---
id: TRX-67
title: Keep plane joins from being walled off by other nets' copper
type: bug
status: in-progress
assignee: claude
priority: high
labels: [planes, verification, detailed-routing]
layer: engine
---

Found on TokenBoard while measuring [[TRX-62]]: traxide routed all 241 nets and `traxide check` found none incomplete, but KiCad 10's DRC with the zones refilled reports one missing connection of VLED. [[TRX-58]] joins a pad to its net's plane with a short track and a via, and counts every via inside the plane's outline as joined. KiCad fills the plane around the copper of other nets, keeping their clearance, and drops or separates what that cuts off: on In2.Cu, the via beside the VLED pad at (320.5, 84.75) mm ended in an island of 3.5 mm², walled off from the rest of the plane by the vias of ROW1, SWCLK and SWDIO within a millimetre of it. The routing before TRX-62 happened not to close that pocket.

- [x] A failing test: a plane via that the vias of other nets enclose is not joined to the plane, in the verification engine; and one in a corridor narrower than its clearance from the zones of other nets
- [x] Verification joins through a plane only what one piece of its fill holds (`fill.rs`, on 0.1 mm cells): the plane's outline within the board, less the larger of its zone's clearance and the rules' around other nets' copper and zones on its layer, the edge clearance from the board's edges and cutouts, shrunk by half the zone's minimum width so that narrower necks part it. The planes keep their zone's clearance and minimum width (`connect_pads (clearance …)`, `min_thickness`). On TokenBoard `traxide check` now agrees with KiCad 10's DRC on three routings: traxide's after [[TRX-62]] 240 of 241 nets (KiCad: one VLED pad cut off), traxide's before 241 (KiCad: nothing unconnected), the author's as before. What walled C148-1's via off was a +3V3 zone of higher priority 0.5 mm south of it, the LED148 cutout north of it and vias of ROW1 and SWCLK east; the zones of other nets' clearance is what first was missing. The checks of the whole board take 2 s
- [ ] Routing keeps a plane via joined: a via of another net that would cut a plane join off from the rest of the plane costs as much as crossing that join, or the pad is routed again by track
- [ ] TokenBoard and the corpus's boards with planes: KiCad's DRC with zones refilled finds no missing connection that `traxide check` does not

**Depends on:** [[TRX-58]]
