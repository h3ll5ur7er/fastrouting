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

- [ ] Routes of a net with fixed wiring start or end at the free ends of its lanes where they can
- [ ] The report's board: no dangling fixed lane

**First try** (local branch `trx-73-lanes`): a search reaches a pad with fixed wiring also at the free ends of that wiring, followed along tracks that meet end to end and through vias, and stopping at other pads of the net; the estimate toward such a pad gives up the lane's reach, so it stays a lower bound. On the report's DSN the route then starts at 44 of the 47 lane ends on the ring around the RP2350, where it started at none. But it costs: 128 s instead of 93 s and 634 vias instead of 578. On TokenBoard's KiCad board, reaching the pad straight costs as much as two vias more: 240 of 241 nets in 55 s; one via more: 241 in 60 s and 563 vias; nothing more: 241 in 48 s and 531 vias, against 241 in 39 s and 523 vias without lanes. KiCad's count of dangling tracks decides which.
