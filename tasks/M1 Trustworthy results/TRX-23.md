---
id: TRX-23
title: Rip up and reroute
type: improvement
status: done
priority: high
labels: [detailed-routing]
estimate: 8
layer: engine
---

Make the detailed router complete real boards.

- Gridless search moved to [[TRX-48]]: completion did not need it
- [x] Negotiated rip-up: a blocked connection may cross routed copper at a price that grows with each rip-up of that connection (history cost), up to `max_rip_ups`; the displaced connections are routed again
- [x] Every test ends with the Verification Engine; zero violations
- [x] The keyboard board routes completely: matrix-6x17, 306 of 306 connections, clean in KiCad 10's DRC (see [[Benchmarks]])

**Depends on:** [[TRX-12]], [[TRX-19]]
