---
id: TRX-60
title: Say why a board cannot be completed
type: improvement
status: in-progress
priority: medium
labels: [manager, cli]
estimate: 2
layer: manager
---

160 of the 489 connections the corpus leaves unrouted cannot be routed legally at all, because of how the board is drawn ([[Corpus-analysis]]). The route report names the pad of each one, but not what is wrong with the board, and some reasons are wrong:

- Issue632-MiniAutoPilot: 151 connections "lie outside the board": 31 of its 36 parts are placed outside its outline. One line should say so, with the parts.
- Issue069-TestSensel (both), Issue110-testPCBSpecctraFile: pads of different nets overlap, so connecting one shorts the other; the report says "no track can leave". Verification skips pad-to-pad conflicts as the footprint's business, so nothing names them.
- Issue433-my-board: the pin R1_source_component_2-2 is listed in two nets; the DSN reader silently keeps the last.
- Issue313-FastTest: the board's protected wiring encloses two pads completely.
- Reasons that mislead: a connection with two causes names only the first, such as a hemmed-in QFP pin whose other pad lies only on a plane layer (Issue269-caniot-tiny-arm, U1-47 → C4-2); once negotiation stops ripping up, a pad whose exits routed copper took is reported as hemmed in, as if fixed copper blocked it (Issue508-DAC2020_bm01, 3 connections that route alone).

- [x] Board Access warns about a pin listed in more than one net, naming both nets (and takes it out of the earlier net, which still listed it); the route and check reports print the warnings, `check --json` lists them
- [x] The route and check reports name the parts that lie wholly outside the outline (Issue632-MiniAutoPilot: its 31 parts in one line)
- [x] They name pads of different nets closer than their clearance (by pair), and pads with no legal way out because of fixed copper or keepouts. Pads of one part count only where they overlap: a footprint may set its pins closer than the board's clearance, and counting those named 110 pairs on Issue145 alone; so the corpus warns on the four boards above and two pairs elsewhere
- [x] "Hemmed in" only when fixed copper, a keepout or the edge blocks the pad; where routed copper took its ways out, the report says so (Issue269-caniot: U1-20 and U1-52); a connection with two blocked pads names both (Issue110-testPCBSpecctraFile: C11-2 and R17-1)
- [x] Tests for each, from small boards built in code; the corpus boards above as examples in [[Corpus-analysis]]
- [ ] Pads enclosed by fixed copper farther out than their exits are named as enclosed, not "no way found" (Issue313-FastTest)
