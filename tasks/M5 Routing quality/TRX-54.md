---
id: TRX-54
title: Leave off-grid pins along their axis
type: improvement
status: todo
priority: high
labels: [detailed-routing]
estimate: 2
layer: engine
---

The largest single limitation found by [[Corpus-analysis]]: 120 connections on 8 boards stay unrouted as "hemmed in", but their pins have a legal way out. Each is a pin of a fine-pitch chip (0.4 or 0.5 mm pitch) whose centre lies off the 0.1 mm grid. Between its neighbours there is room for a track only on the pin's axis: with 0.2 mm pads, track and clearance at 0.4 mm pitch, there is no slack at all. A search can only leave a pad by a straight stub to a grid node inside the pad's bounds or next to its centre (`Search::access` and `Grid::nodes_near` in [[crates/engines/detailed/src/search.rs]], [[crates/engines/detailed/src/grid.rs]]), and every such node lies off the axis. A 0.05 mm grid routes 90 of them alone, but costs four times the states, and the other 30 pins have no slack at all, with centres such as x = 148.501 mm or y = 125.537 mm that no practical grid passes through.

Fix it where the pad is left, not by refining the grid: a stub from the pad's centre straight out along each octilinear direction, off the grid, to the first node beyond the pad's neighbours from which the search can go on. The same stub serves the target end.

- [ ] Failing test first: a QFN-like row of 0.2 mm pads at 0.4 mm pitch with 0.2 mm track and clearance, its centres 0.05 mm (and 0.001 mm) off the grid; each pin connects outward
- [ ] Access ways along the pad's axes to nodes beyond its copper and its neighbours' clearance, at the cost of the stub's length
- [ ] The path keeps the stub as its first and last segment; the octilinear test of [[TRX-47]] still holds
- [ ] Corpus, each with its unrouted count before → after: Issue508-DAC2020_bm01 (42 hemmed in), bm05 (22), bm06 (4), Issue102 and Issue326 Mars-64 (11 each), Issue269-caniot-tiny-arm (28, with [[TRX-57]]), Issue297-myboard (4), Issue157-TeamAdapt-LinePCB (1)
- [ ] The keyboard and the key matrices route as before ([[Benchmarks]])

Freerouting connects bm06 completely and cleanly where traxide leaves these 4 pins; bm05 and bm06 are two of the seven boards where Freerouting connects more ([[Benchmarks#real-boards-the-corpus-against-freerouting-m2-trx-15]]). The free-space graph of [[TRX-48]] would contain the pins' axes anyway; this fix is needed whichever search remains.
