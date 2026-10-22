---
id: TRX-67
title: Keep plane joins from being walled off by other nets' copper
type: bug
status: todo
priority: high
labels: [planes, verification, detailed-routing]
layer: engine
---

Found on TokenBoard while measuring [[TRX-62]]: traxide routed all 241 nets and `traxide check` found none incomplete, but KiCad 10's DRC with the zones refilled reports one missing connection of VLED. [[TRX-58]] joins a pad to its net's plane with a short track and a via, and counts every via inside the plane's outline as joined. KiCad fills the plane around the copper of other nets, keeping their clearance, and drops or separates what that cuts off: on In2.Cu, the via beside the VLED pad at (320.5, 84.75) mm ended in an island of 3.5 mm², walled off from the rest of the plane by the vias of ROW1, SWCLK and SWDIO within a millimetre of it. The routing before TRX-62 happened not to close that pocket.

- [ ] A failing test: a plane via that the vias of other nets enclose is not joined to the plane, in the verification engine
- [ ] Verification joins through a plane only what one piece of its fill holds: the plane's outline less the clearance around other nets' copper on its layer (through-hole pads and vias), as KiCad fills it; `traxide check` then agrees with KiCad's DRC on TokenBoard
- [ ] Routing keeps a plane via joined: a via of another net that would cut a plane join off from the rest of the plane costs as much as crossing that join, or the pad is routed again by track
- [ ] TokenBoard and the corpus's boards with planes: KiCad's DRC with zones refilled finds no missing connection that `traxide check` does not

**Depends on:** [[TRX-58]]
