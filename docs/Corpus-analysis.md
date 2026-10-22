# Corpus analysis

Why traxide leaves real boards incomplete, connection by connection, and which of those connections can be routed legally at all. October 2026, [[TRX-15]]'s corpus (the 76 DSN files of Freerouting's bug reports, [[Benchmarks#real-boards-the-corpus-against-freerouting-m2-trx-15]]), traxide at commit fffa511 with the default profile, 120 s per board, in the 4-core cloud container.

[TOC]

## Summary

40 boards complete, 25 incomplete, 8 stopped at 120 s (Issue066, 103-Board-Unrouted, 113, 145, 208, 219, 289 ×2; speed is [[TRX-48]]'s), 3 fail to load (Issue006, 143, 420; see [[Benchmarks]]). Issue039 and Issue283, which stopped at 120 s on CI's runners, finish here in 31 s and 83 s. The 25 incomplete boards leave **489 connections** unrouted:

| Cause | Connections | Boards | Whose | Fix |
| --- | ---: | ---: | --- | --- |
| [Parts outside the board](#boards-that-cannot-be-completed-legally) | 153 | 3 | the board's | [[TRX-60]] says so |
| [Congestion](#congestion) | 131 | 9 | traxide's: negotiation | [[TRX-55]] (10 of them), [[TRX-56]] |
| [Off-grid fine-pitch pins](#off-grid-fine-pitch-pins) | 120 | 8 | traxide's: leaving a pad | [[TRX-54]] |
| [Pads only on a KiCad power layer](#pads-only-on-a-kicad-power-layer) | 46 | 1 | traxide's: reading the DSN | [[TRX-57]] |
| [Search window too small](#search-window-too-small) | 23 | 10 | traxide's: the search window | [[TRX-55]] |
| [The board's own unlocked wiring](#the-boards-own-unlocked-wiring) | 7 | 2 | traxide's, by choice | [[TRX-59]] |
| [Conflicting pads](#boards-that-cannot-be-completed-legally) | 5 | 4 | the board's | [[TRX-60]] says so |
| [Planes not read](#planes-not-read) | 2 | 1 | traxide's: reading the DSN | [[TRX-58]] |
| [Pads enclosed by locked wiring](#boards-that-cannot-be-completed-legally) | 2 | 1 | the board's | [[TRX-60]] says so |

**160 connections (33 %) cannot be routed legally**: their pads lie outside the outline, overlap another net's pad, belong to two nets, or are walled in by wiring the board locks. The other 329 are traxide's to route. No unrouted connection on any board was a bug in the sense of a broken rule: every session traxide wrote breaks only the rules the board itself already broke.

Per board (unrouted connections; *window* is [[TRX-55]], *off-grid* [[TRX-54]], *illegal* cannot be routed legally):

| Board | Routed | Unrouted by cause |
| --- | ---: | --- |
| Issue029-hw48na | 83/91 | 6 window, 2 congestion |
| Issue039-bug-design | 155/204 | 49 congestion |
| Issue069-TestSensel (both) | 5/6 | 1 illegal: overlapping pads |
| Issue093-interf_u | 2/3 | 1 window (a GND pour joins it, [[TRX-58]]) |
| Issue102, Issue326 Mars-64 | 72/106 | 23 congestion, 11 off-grid |
| Issue110-Pajalnaja_stancija | 104/109 | 2 window, 2 congestion, 1 illegal: part outside the board |
| Issue110-RelayModule | 81/88 | 6 congestion, 1 window |
| Issue110-testPCBSpecctraFile | 2/5 | 2 illegal: overlapping pads, 1 illegal: pad outside the board |
| Issue157-TeamAdapt-LinePCB | 211/214 | 2 unlocked wiring across the pads, 1 off-grid |
| Issue178-KeebMaker_Sofle_Choc | 425/426 | 1 window |
| Issue217-8088sbc | 234/235 | 1 window |
| Issue230-CNH_Functional_Tester (both) | 192/193 | 1 window |
| Issue269-caniot-tiny-arm | 3/89 | 46 power layer, 28 off-grid, 6 window, 5 unlocked wiring, 1 congestion |
| Issue269-z10_module | 63/68 | 3 window, 2 planes |
| Issue283-UnconnectedTracesUnderPads | 178/200 | 22 congestion |
| Issue297-myboard | 157/161 | 4 off-grid |
| Issue313-FastTest | 1/3 | 2 illegal: enclosed by locked wiring |
| Issue433-my-board | 8/9 | 1 illegal: a pin in two nets |
| Issue508-DAC2020_bm01 | 153/195 | 39 off-grid, 3 congestion |
| Issue508-DAC2020_bm05 | 85/107 | 22 off-grid |
| Issue508-DAC2020_bm06 | 94/98 | 4 off-grid |
| Issue632-MiniAutoPilot | 16/167 | 151 illegal: parts outside the board |

## How the connections were classified

`traxide route` names a reason for every unrouted connection: a pad off the board, a pad only on layers not routed, a pad hemmed in (no straight stub from it reaches a grid node), or no way found. Each connection was then routed again **alone**, on the board with only its own fixed copper, with the router's own search (a throwaway program on the workspace's crates), in four settings until one routed it: the default profile; a whole-board window (`detour_margin` 100 mm); a 0.05 mm grid; a 0.025 mm grid. A connection that routes alone with the default profile failed for **congestion**: the other connections took its room. One that needs a wider window or a finer grid points at that limit. One that routes in no setting was examined by hand: the pad's neighbours and their gaps, its eight straight exits, the keepouts and the board's own copper, drawn as SVG.

Then the remedies were tried on whole boards where a profile can express them (`detour_margin`, `grid_pitch`, rip-up and history settings), and on a copy of a DSN where the reading of a layer was the question. Freerouting 2.5.0 ran as a black box on the congested boards.

## Off-grid fine-pitch pins

120 connections, all reported "no track can leave": pins of QFPs and QFNs at 0.4 or 0.5 mm pitch. Between two neighbours, a track fits only along the pin's axis: on Issue508-DAC2020_bm05's U27 (0.2 mm pads at 0.4 mm pitch, 0.2 mm track and clearance) with no slack at all, on Issue269-caniot's U1 (0.3 mm pads at 0.5 mm pitch, 0.25 mm track, 0.2 mm clearance) with 0.025 mm. The pins' centres lie off the 0.1 mm grid (U27-44 at x = 148.501 mm, caniot's U1 at y = 59.75 mm), and a search leaves a pad only by a straight stub to a grid node inside the pad or next to its centre: every such node is off the axis, so every stub comes too close to a neighbour. The straight exits from the centre are clear (U27-44: 2.8 mm north into the chip, U1-3: 3.55 mm east and west).

A 0.05 mm grid routes 90 of them alone; for the other 30 (Issue508-bm05 and bm06, Issue297) no practical grid passes through the axis. The fix belongs where the search leaves a pad: a stub along the pad's axis to the first node beyond its neighbours, off the grid ([[TRX-54]], in [[crates/engines/detailed/src/search.rs]]). Freerouting routes bm06 completely and cleanly.

Since this analysis, a search also leaves every pad along its axes, off the grid ([[Benchmarks#off-grid-pins-left-along-their-axis-m5-trx-54]]): of the 130 connections these boards reported hemmed in, 12 still are; bm06 is complete, bm01 routes 169 connections of 195, bm05 99 of 107, Issue297 160 of 161, the Mars-64 boards 83 of 106.

## Search window too small

23 connections route alone only when the search may cover the whole board: each search sees the box around its pads grown by `detour_margin`, 5 mm, and the way round a header or the board's wiring is longer. In the full run the window also cost connections that collided with them. Whole boards with `detour_margin = 100`, every session clean:

| Board | 5 mm | 20 mm | 100 mm |
| --- | ---: | ---: | ---: |
| Issue029-hw48na | 83/91 | 81/91 | **91/91** |
| Issue093-interf_u | 2/3 | 2/3 | **3/3** |
| Issue110-RelayModule | 81/88 | **88/88** | **88/88** |
| Issue110-Pajalnaja_stancija | 104/109 | 108/109 | 108/109 |
| Issue178-KeebMaker_Sofle_Choc | 425/426 | **426/426** | **426/426** |
| Issue217-8088sbc | 234/235 | **235/235** | **235/235** |
| Issue230-CNH_Functional_Tester (both) | 192/193 | **193/193** | **193/193** |
| Issue269-z10_module | 63/68 | 64/68 | 65/68 |

Seven boards complete, among them Issue217, which Freerouting completed only by breaking rules. A wide window for every search would make the long searches of every board slower; [[TRX-55]] widens it only for a search that failed.

## Congestion

131 connections on 9 boards route alone with the default profile but not together with the rest: Issue039-bug-design (49), the two Mars-64 boards (23 each), Issue283-UnconnectedTracesUnderPads (22), Issue110-RelayModule (6), Issue029, Issue110-Pajalnaja (2 each), Issue508-DAC2020_bm01 (3, then reported as hemmed in: once negotiation stops, routed copper took the pins' exits; the report now says so) and Issue269-caniot (1). The wider window of [[TRX-55]] settles the 10 on Issue029, RelayModule and Pajalnaja; the rest are lost in negotiation.

Rip-up and reroute stops ripping up once twice as many rip-ups as there are connections have passed without more connections routed; the connections still queued then search without crossing anything, and fail. That rule ends negotiation on every congested board (Issue039: 472 rip-ups for 204 connections; Mars-64: 412 for 106). No profile setting helps:

After this analysis, negotiation ended after four rounds without gain instead of two, and since fine-pitch pins escape first it ends after two again ([[TRX-56]]) ([[Benchmarks#negotiation-that-goes-round-in-circles-m2-trx-48]]): the Mars-64 boards route 78 connections of 106 again (71 of 94 nets), as without an end; the gap to Freerouting's 94 of 94 stays.

| Profile change | Issue039 (204) | Mars-64 (106) | Issue283 (200) | RelayModule (88) |
| --- | ---: | ---: | ---: | ---: |
| none (default) | 155 | 72 | 178 | 81 |
| `max_rip_ups = 100` | 155 | 72 | 176 | 81 |
| `history_cost = 0` | 155 | 69 | 182 | 79 |
| `history_cost = 5` | 164 | 74 | 179 | 83 |
| `heuristic_weight = 1.0` | 161 | 75 | 179 | 87 |
| `via_cost = 0.5` | 157 | 75 | 171 | 78 |
| `batch_size = 1` | 150 | 75 | 179 | 80 |
| `grid_pitch = 0.05` | 155 | 73 | 175 | 80 |

Freerouting 2.5.0, run as a black box with 900 s on the same DSN files and judged by `traxide check`, does much better: Mars-64 (Issue326) all 94 nets against traxide's 62, breaking only 6 edge clearances; Issue039 64 of 74 nets against 32, with 6 clearance violations; Issue283 55 of 58 against 42, with 17 edge violations. Its log shows a fan-out stage that escapes the SMD pins first, then passes over all unrouted items with rising rip-up costs. traxide's routes on these boards wander and spend vias (Mars-64: 199 vias for 72 connections, Freerouting 155 for all), taking the room the last connections need. Negotiating in rounds, PathFinder's way, and escaping fine-pitch pins first are [[TRX-56]]; tidier routes would also come from [[TRX-36]].

## Pads only on a KiCad power layer

Issue269-caniot-tiny-arm is a 2-layer board whose B.Cu KiCad exported as `(type power)`, without any `(plane …)`. traxide routes no track on a power layer, but the board's bottom-side SMD parts (capacitors, fuses, a diode) have copper only there: 46 connections. KiCad itself routes and checks tracks on power layers like on signal layers. With B.Cu read as a signal layer the board routes 48 of 89 connections instead of 3, clean; with a 0.05 mm grid as a stand-in for [[TRX-54]] 70, with a whole-board window as well 75 ([[TRX-57]]).

## Planes not read

19 corpus boards state copper planes, `(plane NET (polygon LAYER …))`; traxide ignores them. On Issue269-z10_module, GND is a plane on In1.Cu and +3V3 on In2.Cu: a via next to each pad would connect them, but traxide routes GND as tracks, and two of them (R103-2 → IC101-28, IC101-28 → IC101-42) have no way on the outer layers at all. On Issue093-interf_u a GND pour on the bottom joins the one connection left ([[TRX-58]]).

Since this analysis, planes are read ([[Benchmarks#pads-joined-through-their-nets-planes-m5-trx-58]]): on z10_module the planes join all but two GND pads, IC101-28 and IC101-42, for which no via fits beside them; Issue093's bottom pour is routed as before, and the board is complete anyway.

## The board's own unlocked wiring

traxide keeps all wiring a board comes with. Specctra's `(type route)` wiring, which KiCad writes for unlocked tracks, may be moved by a router; ten corpus boards carry some. On Issue269-caniot, with one routable layer, the board's 335 tracks wall in J8-9 and the QFP's pins 51–53 (5 connections, no way at any grid); on Issue157-TeamAdapt-LinePCB a track of Net-(D49-Pad1) runs across the pads Q1-3 and Q2-3 of other nets, breaking the rules already, and leaves them no way out (2). Moving a designer's tracks should be their choice ([[TRX-59]]).

## Boards that cannot be completed legally

160 connections on 7 boards. Freerouting connects some of them by breaking rules: all 211 nets of Issue632, with 520 violations.

- **Parts outside the board** (153). Issue632-MiniAutoPilot: the outline covers 37.6 × 69.1 mm, and 31 of the board's 36 parts are placed outside it, most at y ≈ −116 mm in the DSN's coordinates, where the outline spans +327 to +396 mm: 151 connections. Issue110-Pajalnaja_stancija: C9, a 10 mm capacitor, lies outside the outline. Issue110-testPCBSpecctraFile: the edge connector's pad XP5-2 reaches only about 0.3 mm into the board, less than the 0.5 mm edge clearance KiCad keeps by default.
- **Conflicting pads** (5). Issue069-TestSensel (both files): KiCad exported the interdigitated sensor pads as overlapping quadrilaterals, so J4-1 lies under J4-2 of another net. Issue110-testPCBSpecctraFile: C11-2 overlaps the GND pad XP4-2. Issue433-my-board: the pin R1_source_component_2-2 is listed in two nets; the DSN reader keeps the last one, and joining the other would short them.
- **Enclosed by locked wiring** (2). Issue313-FastTest, a tiny board in mils (0.045 mm tracks), whose protected wiring encloses two pads completely: no way even on a 0.0025 mm grid.

Verification does not count pad-to-pad conflicts as broken rules (the footprint's business). Since [[TRX-60]], `traxide check` and `traxide route` warn of them instead, pair by pair. Pads of one part are named only where they overlap, as a footprint may set its pins closer than the board's clearance. So the corpus warns of the four pairs of TestSensel (both files), XP4-2 with C11-2 and R17-1 on Issue110-testPCBSpecctraFile, and one pair each on Issue163 and Issue367-Charger. The route report names each pad of an unrouted connection that no track can leave. It also tells fixed copper that hems a pad in from the routed copper of other nets that took its ways out (Issue269-caniot: U1-20 and U1-52). Where even a search through the routed copper finds no way, the report says the board itself leaves none (Issue313-FastTest: both of its connections).

## Redoing the analysis

`cargo xtask corpus` downloads the boards into `target/corpus`; `traxide route BOARD.dsn -o OUT.ses` without `--quiet` lists every unrouted connection with its reason. Routing one connection alone takes a few lines on the workspace's crates: `Layout::build` on the board, then `traxide_detailed::Router::route_plan` with a `Plan` of that one connection and the profile to try. The pictures of the problem areas were drawn from the `Design` as SVG by such a throwaway program, not kept.
