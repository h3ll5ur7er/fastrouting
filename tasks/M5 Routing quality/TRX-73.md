---
id: TRX-73
title: Continue fixed wiring from its free end
type: improvement
status: todo
priority: low
labels: [detailed-routing, dsn]
layer: engine
---

From the TokenBoard report ([[Research/TokenBoard-report]]): the RP2350's fan-out is fixed wiring, a lane from each pin to a ring 4.6 mm out, meant to be continued from its free end. On 20 nets traxide joined the net elsewhere, so the lane's free end stays dangling (an antenna stub; the nets are complete).

- [ ] Routes of a net with fixed wiring start or end at the free ends of its lanes where they can
- [ ] The report's board: no dangling fixed lane
