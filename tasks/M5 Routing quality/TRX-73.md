---
id: TRX-73
title: Continue fixed wiring from its free end
type: improvement
status: in-progress
assignee: claude
priority: low
labels: [detailed-routing, dsn]
layer: engine
---

From the TokenBoard report ([[Research/TokenBoard-report]]): the RP2350's fan-out is fixed wiring, a lane from each pin to a ring 4.6 mm out, meant to be continued from its free end. On 20 nets traxide joined the net elsewhere, so the lane's free end stays dangling (an antenna stub; the nets are complete).

- [x] Routes of a net with fixed wiring start or end at the free ends of its lanes where they can
- [ ] The report's board: no dangling fixed lane (2 of the 47 lane ends on the ring left)

**First try:** a search reaches a pad with fixed wiring also at the free ends of that wiring, followed along tracks that meet end to end and through vias, and stopping at other pads of the net; the estimate toward such a pad gives up the lane's reach, so it stays a lower bound. With the pad's own ways two vias dearer, the route on the report's DSN starts at 44 of the 47 lane ends on the ring around the RP2350, where it started at none. But it costs: 128 s instead of 93 s and 634 vias instead of 578. On TokenBoard's KiCad board, reaching the pad straight costs as much as two vias more: 240 of 241 nets in 55 s; one via more: 241 in 60 s and 563 vias; nothing more: 241 in 48 s and 531 vias, against 241 in 39 s and 523 vias without lanes.

**Chosen:** a pad's free ends are ways in beside the pad's own, at no extra cost: a route continues a lane where that is the shorter way. Dangling tracks in KiCad's DRC of TokenBoard: 29 before; 5 at no extra cost (241 nets, 43 s, 531 vias); 4 at one via extra (60 s, 563 vias). On the report's DSN: 392 of 392 in 65 s (93 s before), 607 vias (578), 2 of the 47 ring lanes left dangling. The first route of TokenBoard so came within 0.8 µm of LED177's cutout, KiCad found: [[TRX-74]].
