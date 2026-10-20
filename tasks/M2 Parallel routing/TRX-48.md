---
id: TRX-48
title: Search free space instead of a fine grid
type: improvement
status: in-progress
priority: high
labels: [detailed-routing, performance]
estimate: 8
layer: engine
---

The detailed search walks a 0.1 mm grid: millions of states on a large board, most of them in open space where nothing needs deciding. Searching a graph of the places where something does change — corners of obstacles inflated by their clearance, the corridor's crossing points, a coarse grid in between — needs far fewer states for the same paths ([[Research/Algorithms#classical-baselines]]).

- [ ] Implicit octilinear graph from corners of inflated obstacles plus a fallback grid
- [ ] Same results as the grid search on the end-to-end boards, measured in [[Benchmarks]]
- [ ] Decide after [[TRX-26]]: corridors may shrink the grid search enough

Measured in [[TRX-27]]: the keyboard's time is in its few long connections, which overlap everything and so cannot run side by side. The worst, from the MCU across the board, popped 4.4 million of its window's 17.7 million grid states and took 2 s, mostly expanding states again ([[TRX-49]] fixed that: 0.77 s); most searches touch under 2 % of their window.

Measured on real boards ([[TRX-15]]'s corpus), October 2026: on Issue029-hw48na, the long searches cost 1.1–4 times their straight-line estimate and flood up to 1.8 million states each. A coarse distance map of the fixed copper (0.5 mm cells, Dijkstra from the target) as the estimate barely raised it (24 → 34 mm where 98 mm were paid) and made nothing faster: what makes those paths dear is negotiation, crossing routed copper, not detours around fixed copper. The outline index, this task's first step, made each expansion three times cheaper on boards with rounded corners.

Searching free space first, and with rip-ups only where it finds no way, measured October 2026: Issue110-RelayModule routed 85 of 88 connections in 11 s instead of 81 in 40 s, but Issue034-Green14SegLED lost its completeness (144 of 147 instead of 147) and the keyboard took 2.3 s instead of 1.9 s. Free space first while paying history costs did worse (142 of 147, Issue015 no faster). Negotiation's flooding stays: where the only ways cross routed copper, the search pays rip-up costs its straight-line estimate does not know.

A heavier estimate, measured October 2026 against today's 1.2: at 1.5 the keyboard routed in 0.64 s instead of 1.45 s but with 128 vias instead of 113, Issue015 in 30 s instead of 35 s with 10 % more vias, and Issue110 took twice as long; at 2.0 Issue034 lost its completeness (143 of 147). The default stays 1.2; a heavier one would suit only a profile that trades vias for time.

What the flooding searches pay, measured on Issue015 in October 2026: the largest estimated 72–88 mm and paid 105–250 mm, of which the length was 84–103 mm, 3–11 vias at 2 mm each (the estimate allows at most one), and on some searches 17–143 mm of rip-up and history costs. A coarse cost-to-target map that knows on which layer routed copper lies, and what a via costs, could capture most of that; the map of fixed copper alone could not.

Regression to look into: ending negotiation after two rounds without gain leaves the Mars-64 boards at 62 of 94 nets instead of 71.

Done so far: negotiation that ends when it goes round in circles ([[Benchmarks#negotiation-that-goes-round-in-circles-m2-trx-48]]), far polygon edges passed over at a glance (Issue110 in 40 s instead of 132 s), an indexed outline (three times cheaper expansions on boards with rounded corners) and one search state per node and layer instead of nine (up to 2.5 times faster), [[Benchmarks#one-state-per-node-m2-trx-48]].

**Depends on:** [[TRX-23]]
