# Benchmarks

Measurements of traxide on the end-to-end boards, newest first; the comparison with Freerouting runs in CI, the rest by hand until the benchmark gate of [[TRX-22]]. Every board traxide routed below was imported into KiCad 10.0.7 and passed its DRC with no violations.

[TOC]

## How to measure

```bash
cargo xtask corpus --timeout 60   # 76 real boards from Freerouting's bug reports, downloaded at run time
cargo xtask corpus --freerouting "java -jar freerouting.jar" --shard 1/6   # the same, against Freerouting; CI runs 6 shards
cargo xtask e2e placements crossing matrix-3x4 matrix-6x17   # builds, routes and checks the recipes
time target/release/traxide route -q target/e2e/matrix-6x17/matrix-6x17.dsn
```

The machine: a 4-core cloud container (x86-64), one thread used.

## traxide against Freerouting 2.5.0

October 2026, `cargo xtask bench` in the Benchmark workflow on a GitHub Actions runner (ubuntu-latest, 4 vCPUs): both routers get the same DSN exported by KiCad 10, KiCad 10 imports each session, and KiCad's own DRC judges both. Freerouting ran with its defaults (`-de board.dsn -do board.ses --gui.enabled=false`); traxide with its default profile. Two runs on different days' runners:

| Board | Router | Time | Unconnected | Violations | Tracks | Vias | Track length |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| matrix-6x17 | **traxide** | **1.4 s / 1.0 s** | 0 | **0** | 547 | **23** | **5947 mm** |
| matrix-6x17 | Freerouting | 54.4 s / 90.0 s | 0 | 30 / 18 | 852 / 831 | 40 / 36 | 6605 / 6662 mm |
| keyboard-tkl | **traxide** | **9.8 s / 7.2 s** | 0 | **0** | 1552 / 1550 | **108** | **8304 mm** |
| keyboard-tkl | Freerouting | 266.2 s / 480.9 s | 0 | 199 / 199 | 1645 / 1577 | 225 / 230 | 8548 / 8574 mm |

**After the speed work of M2** ([[TRX-49]], [[TRX-24]], [[TRX-50]]), the same benchmark on a CI runner:

| Board | Router | Time | Unconnected | Violations | Tracks | Vias | Track length |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| matrix-6x17 | **traxide** | **0.5 s** | 0 | **0** | 561 | **19** | **5942 mm** |
| matrix-6x17 | Freerouting | 29.2 s | 0 | 7 | 831 | 47 | 6672 mm |
| keyboard-tkl | **traxide** | **1.5 s** | **0** | **0** | 1616 | **107** | **8279 mm** |
| keyboard-tkl | Freerouting | 105.3 s | 1 | 199 | 1647 | 244 | 8345 mm |

Freerouting's times vary a lot between runners (29–90 s and 105–481 s over three runs); traxide's routes never change. In the first two runs, traxide was 39–90 times faster on the matrix and 27–67 times on the keyboard, with about half the vias and less track, and its boards are the ones KiCad finds clean. traxide's routes are the same in every run; Freerouting's differ from run to run.

**What Freerouting's violations are.** In the second run KiCad listed them by kind: all 18 on the matrix and all 199 on the keyboard are `track_width`, with no clearance violation and nothing unconnected. They are tracks narrower than 0.2 mm, the minimum track width of KiCad 10's default board setup, which the recipes keep, and the width the DSN asks for. So they come from Freerouting's own choice of narrower tracks, not from a rule the DSN leaves out; the KiCad edge clearance that tripped traxide early on ([[Research/KiCad-integration#rules-the-dsn-leaves-out]]) did not trip Freerouting here. The Benchmark workflow now keeps the sessions, DRC reports and routed boards as an artifact, to show where those tracks are.

## Each edge checked once, and copper queries no wider than copper clearances (M2, TRX-48)

Two changes with byte for byte the same routes: an edge of the grid is checked once for both directions instead of once from each end, and a query for copper near a track or via looks as far as the largest clearance between copper, no longer as far as the board's edge clearance, which KiCad sets to 0.5 mm and the outline check covers on its own.

| Board | Before | **After** |
| --- | ---: | ---: |
| keyboard-tkl, complete | 1.84 s | **1.45 s** |
| Issue034-Green14SegLED, complete | 40 s | **25 s** |
| Issue015-StackOverflow, complete | 40 s | **35 s** |
| Issue110-RelayModule, 81 of 88 | 40 s | **37 s** |

## Far edges skipped at a glance (M2, TRX-48)

Stack samples of Issue110-RelayModule showed nearly all of its time in checking a track's clearance to pads drawn as polygons of dozens of edges, each edge with the exact test, at 33 µs per node expanded. Two segments whose bounding boxes lie the distance apart along an axis are now passed over at once, a polygon keeps its bounding box, and a shape far from a polygon skips its edges altogether. The routes are byte for byte the same:

| Board | Before | **After** |
| --- | ---: | ---: |
| Issue110-RelayModule, 81 of 88 | 132 s | **40 s** |
| Issue015-StackOverflow, complete | 46 s | **40 s** |
| keyboard-tkl, complete | 1.8 s | **1.9 s** |

## Negotiation that goes round in circles (M2, TRX-48)

On congested corpus boards the routed count rose, then swung back and forth: Issue039 between 135 and 151 of 204 connections for as long as it was let run, as connections took each other's ways in turn, each up to 20 times. Negotiation now ends once two rounds of rip-ups, as many as there are connections each, have routed no more connections than the best so far; the connections left search without ripping up others. Measured on this machine:

| Board | Before | One round | **Two rounds** | Three rounds |
| --- | --- | --- | --- | --- |
| Issue039-bug-design | > 150 s | 28 s, 158 of 204 | **54 s, 155 of 204** | 84 s, 153 of 204 |
| Issue110-RelayModule | > 150 s | 97 s, 77 of 88 | **134 s, 81 of 88** | |
| Issue034-Green14SegLED | 40 s, 147 of 147 | 28 s, 144 of 147 | **41 s, 147 of 147** | 42 s, 147 of 147 |
| Issue029-hw48na | 66 s, 83 of 91 | 45 s, 82 of 91 | **68 s, 83 of 91** | |
| keyboard-tkl | 1.9 s, complete | 1.9 s, complete | **1.9 s, complete** | |

One round cost Issue034 three connections; two keep every board as complete as before and end the circling.

Two turned out too few for the Mars-64 boards of the corpus (62 of 94 nets instead of 71). Measured again with four:

| Board | Two rounds | **Four rounds** |
| --- | --- | --- |
| Issue102-Mars-64 | 22 s, 72 of 106 | **83 s, 78 of 106** (as many as without an end) |
| Issue039-bug-design | 54 s, 155 of 204 | **65 s, 160 of 204** |
| Issue110-RelayModule | 37 s, 81 of 88 | **74 s, 77 of 88** |
| Issue029-hw48na | 46 s, 83 of 91 | **46 s, 83 of 91** |

Four route the most in all (398 connections against 391) and stay within the corpus's 120 s, so negotiation now ends after four rounds without gain. The counts move unevenly between settings: where negotiation churns, which connection wins a contested place decides much. The boards stay slow for another reason: single searches of up to half a second each where the board is crowded ([[TRX-48]]).

## Real boards: the corpus against Freerouting (M2, TRX-15)

October 2026, `cargo xtask corpus --freerouting … --timeout 120` in the Benchmark workflow, six shards on GitHub Actions runners (4 vCPUs each), on commit 7534cf5. The 76 boards are the DSN files of Freerouting's own bug reports. Both routers get the same DSN and 120 s; `traxide check` judges both sessions alike, counting only the rules a session breaks beyond those the board already broke.

| Router | Complete | Incomplete | Breaking rules | Stopped at 120 s | Failed | Time in all |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| **traxide** | **40** | 23 | **0** | **10** | 3 | **1806 s** |
| Freerouting 2.5.0 | 15 | 5 | 19 | 32 | 5 | 4969 s |

- **traxide broke no rule on any board.** Freerouting's sessions break rules on 19 boards, up to 520 violations on Issue632-MiniAutoPilot.
- **Where both finished, traxide was 10–100 times faster**: the three Z80 boards (529 nets) in 0.2 s against 21.6 s or more than 120 s; Issue230 (764 nets) in 6 s against 62 s; Issue199 in 0.2 s with all 28 nets against 109 s with 1.
- **Where Freerouting connected more**, seven boards, it broke rules to do so on six: Issue632 (211 of 211 nets with 520 violations, against traxide's 124 of 211, clean), Issue508-bm05, Issue110-Pajalnaja, Issue217 (104 of 104 with 6 violations, against 103 of 104) and both Issue069 boards. Issue508-bm06 (38 of 38 against 35 of 38, both clean) is the exception.
- **traxide's gaps**: 10 boards stop at 120 s (large or congested ones: Issue039, 066, 103-Unrouted, 113, 145, 208, 219, 283, 289 ×2; [[TRX-48]]), and some finish incomplete where a pad is hemmed in or no way was found (Issue269-caniot 47 of 94, Issue102/326-Mars-64 71 of 94). Why, connection by connection, and which of them can be routed legally at all: [[Corpus-analysis]].
- **Failures**: Issue006 is a schematic, not a design; Issue143-rpi_splitter_mod names a component that does not exist; Issue420 names a pin its component lacks. Three of Freerouting's five failures are the judge's: its sessions define their own via padstacks, which `traxide check` did not read ([[TRX-15]]). It reads them now: Freerouting renames the padstacks of the vias it places (`Round1$13.779528` becomes `Round1$13`, `Via[0-1]_1016:485.7_um` becomes `Via[0-1]_1016:485_um`), so the judge takes each for the design's via with the same copper, within the session's resolution. Judged so, Freerouting 2.5.0 completes Issue229 (30 of 30 nets, clean), Issue143-rpi_splitter and Issue270 (5 of 5 nets, 2 clearances broken each, see below).

The judge is traxide's own Verification Engine, which reads the rules from the DSN as the router does; KiCad's DRC is the independent judge only for the KiCad-built boards above. It reads only a board's general clearance, not the clearances of a type such as `(clearance 6 (type wire_via))`, so it may be stricter than such a board: the clearances Freerouting breaks on Issue143 and Issue270 all involve its vias (without them none is left), and those boards allow 6 mil between a via and a wire, pin or via, where their general clearance is 12 mil; on Issue143 a track passes 6.0 mil from a via.

**After the speed work of TRX-48** (commit efc52c4, same setup):

| Router | Complete | Incomplete | Breaking rules | Stopped at 120 s | Failed | Time in all |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| **traxide** | 40 | 27 | **0** | **6** | 3 | **1323 s** |
| Freerouting 2.5.0 | 17 | 7 | 16 | 31 | 5 | 4863 s |

Four boards that stopped now finish (Issue039 in 19 s, Issue113, Issue145 and Issue283 short of complete); Issue022 routes in 45 s instead of 76 s, Issue178 in 12 s instead of 69 s. One regression: the two Mars-64 boards finish in 18 s instead of 111 s, with 62 of 94 nets instead of 71: negotiation ends too early there ([[TRX-48]]).

## One state per node (M2, TRX-48)

October 2026: on the real boards of the corpus, the long searches expanded about 8 states per node and layer: the heading a node was entered with was part of the search state, and as a bend costs only 0.1 mm, all nine ways into a node cost nearly the same and were each expanded. A state is now a node on a layer; its heading is the direction of the cheapest way in found, which bend costs and the rule against acute turns follow. Every board stays clean in KiCad 10's DRC; track lengths are within 0.2 % of before.

| Board | Before | One state per node | And one check around open nodes |
| --- | ---: | ---: | ---: |
| matrix-6x17 | 0.9 s | 0.35 s | **0.25 s** |
| keyboard-tkl | 3.3 s | 2.6 s | **2.2 s** |
| Issue015-StackOverflow (corpus) | 99 s | 72 s | **50 s** |
| Issue029-hw48na (corpus) | > 120 s | 103 s, 83 of 91 | **72 s**, 83 of 91 |

Then most nodes of a search lie in open space: one check of a disk around a node that covers its eight edges replaces eight checks when nothing is in reach, with the same routes.

## History costs (M2, TRX-50)

October 2026: after the new ordering, the keyboard's MCU connections still ripped each other up 96 times, nearly all within a few millimetres of the QFN's pins. Rip-up prices only protect the connection ripped up; negotiated congestion also remembers *where* routes collide. Every segment or via of a committed route that displaced others now makes its 0.5 mm cell dearer by `history_cost` for every later search ([[Profiles]]).

| `history_cost` | keyboard-tkl | Vias |
| ---: | ---: | ---: |
| 0 (before) | 7.4 s | 117 |
| 0.5 mm | 6.2 s | 135 |
| 1 mm | 4.2 s | 134 |
| 1.5 mm | 3.7 s | 119 |
| **2 mm (the default, a via)** | **3.4 s** | **107** |
| 2.5 mm | 3.4 s | 107 |
| 3 mm | 4.2 s | 117 |
| 5 mm | 8.0 s | 139 |

One run each, all cores of the 4-core container. The key matrices route as before. With the default, `cargo xtask bench` in the container: keyboard-tkl in 3.4 s, 315 of 315 connections, 107 vias, 8279 mm of track, clean in KiCad 10's DRC with nothing unconnected; the routes are the same on 1 and 4 threads.

## Hemmed-in pads first (M2, TRX-24)

October 2026: after [[TRX-49]], 83 rip-ups remained on the keyboard, all among the connections from the MCU's matrix pins, routed last because they are long: each pin's way out of the 0.4 mm pitch QFN was blocked by the copper of its neighbours. Planning now routes connections from **hemmed-in pads** first, pads with another net's copper too close for a track to pass between them, then the rest shortest first. Measured on the same DSN, three runs each, all cores of the 4-core container:

| keyboard-tkl | Time (median) | Tracks | Vias | KiCad 10 DRC |
| --- | ---: | ---: | ---: | --- |
| Shortest first | 12.6 s | 423 | 109 | clean |
| Hemmed-in pads first | **6.6 s** | 432 | 117 | **clean, nothing unconnected** |

The key matrices have no hemmed-in pads and route as before.

**Same board, same routes.** KiCad exports the parts of a board in a different order each time, and planning broke ties by that order: two exports of the keyboard routed differently, and one of them left a connection unrouted after its 10 rip-ups. Planning now breaks ties by names, so both exports give the same routes, and `max_rip_ups` is 20 ([[Profiles]]). The rip-up price made no difference: 5 and 10 mm gave identical routes.

## Each state expanded once (M2, TRX-49)

October 2026: a weighted search reaches many states first by a worse way round, and expanded them again for every better way: the keyboard's longest search expanded 3.7 million times, but only 590 000 distinct states. Now each state is expanded once, which keeps the path within the weight of the cheapest. Measured in the 4-core container, every board clean in KiCad 10's DRC with nothing unconnected:

| Board | Time before | Time after | Vias | Track length |
| --- | ---: | ---: | ---: | ---: |
| matrix-3x4 | 0.08 s | 0.08 s | 4 → 4 | 637 mm |
| matrix-6x17 | 1.65 s | **0.78 s** | 23 → **19** | 5947 → 5943 mm |
| keyboard-tkl | 13.7 s | **12.6 s** | 108 → 109 | 8304 → 8334 mm |

Each search of the keyboard's longest connection is now three times faster (0.77 s instead of 2.2 s), but the long connections out of the MCU now take turns ripping each other up, 5 to 8 searches each, for the QFN's escape room: planning their escape ([[TRX-24]]) comes next.

## Parallel batches (M2, TRX-27)

October 2026: detailed routing searches batches of connections side by side, connections whose territories (search window plus copper reach plus clearance) are apart, and commits them in queue order ([[Architecture/Parallelism-and-GPU#batches]]). Measured in the 4-core cloud container, three runs each, median, default profile (`batch_size = 16`):

| Threads | keyboard-tkl | matrix-6x17 |
| ---: | ---: | ---: |
| 1 | 13.7 s | 1.65 s |
| 2 | 13.6 s | 1.64 s |
| 4 | 13.4 s | 1.61 s |

The sessions were byte-identical in all nine runs of each board, and identical to routing one connection at a time. That correctness is the gain: **no speed-up** on these boards, for two reasons found by tracing every batch:

- **The time is in the long connections.** On the keyboard, 89 of 125 batches held a single connection, and those took 10.8 s of the 12.2 s of searching: connections from the MCU or the header across the board, whose windows overlap everything. The connections that did run side by side took under a second together. One connection alone, searched twice after rip-ups, costs 4 s: it pops 4.4 million of its window's 17.7 million states.
- **Memory.** The search arrays covered each window densely, over a gigabyte for the longest searches, and every thread needed its own. Filling fresh memory for each thread made four threads 15–30 % *slower* than one. The arrays are now tiled, and a search maps only the tiles it writes: peak memory fell from 1.15 GB to 293 MB on one thread (2.2 GB to 365 MB on four), and extra threads cost nothing. On one thread, the tiles cost a few per cent of search speed.

Batches will matter on boards with many short connections spread out; on keyboards, cheaper long searches come first ([[TRX-48]]). A first attempt that searched the next 8–32 connections speculatively, regardless of overlap, and searched colliding ones again, was 2–4 times *slower*: the long connections all converge on the MCU and kept colliding.

## Global routing corridors: measured, and off (M2, TRX-26)

October 2026: the Global Routing Engine plans a corridor for every connection on a grid of cells four track pitches wide (1.4 mm on the keyboard), with negotiated congestion; detailed routing then keeps inside the corridor, or searches the whole window when the corridor turns out to be blocked.

| Detailed routing | keyboard-tkl | matrix-6x17 |
| --- | --- | --- |
| Without corridors (the default) | **17.7 s**, 315 of 315 | **2.7 s**, 306 of 306 |
| Inside corridors | 153 s, 310 of 315 | 4.5 s, 306 of 306 |
| Corridors as guidance (steps outside cost double) | 182 s, 315 of 315 | 5.5 s, 306 of 306 |

Global routing itself takes 0.1–0.6 s. The time it costs goes into the detailed searches: the slow connections are not slow for want of a coarse plan but in their last millimetres into a crowded pin field, which a 1.4 mm grid cannot see; inside a narrow corridor such a search can only rip up instead of detouring. Corridors stay available (`global_routing = true` in a profile, see [[Profiles]]) and may pay off on larger boards; the next attempts are a lane model from the real copper instead of bounding boxes, and corridors that only guide the order of searches.

## The keyboard: a 4-layer RP2350 ISO TKL (M1)

The board traxide was started for, as `e2e/kicad/boards.py` builds it (`keyboard-tkl`): 88 MX switches in the ISO tenkeyless layout with a diode each (NKRO, 6 × 17 matrix), an RP2350-like MCU (QFN-60, 0.4 mm pitch) with USB-C, series resistors, QSPI flash, crystal, regulator and decoupling, on four signal layers; tracks 0.2 mm, clearance 0.15 mm.

| Board | Connections | Routed | Tracks | Vias | Time | KiCad 10 DRC |
| --- | --- | --- | --- | --- | --- | --- |
| keyboard-tkl | 315 | **315** | 423 | 109 | **17.7 s** | **clean, nothing unconnected** |

![keyboard-tkl routed by traxide, imported into KiCad 10: top copper red, inner layers orange and green, bottom copper blue](images/keyboard-tkl.png)

![The RP2350's fan-out: 0.4 mm pitch pins escaping on all four layers](images/keyboard-tkl-mcu.png)

What it took, each found by routing this board:

- **Search from the tighter end.** The first attempt never finished: searches into the MCU's 0.4 mm pitch pins flooded the board, because the way into a hemmed-in pin is far longer than the straight distance. Searching from the end with fewer ways in or out (the pin, 9 access states, against 1936 for a switch pad) floods only the pocket around the pin.
- **Keepouts of footprints.** KiCad exports a footprint's non-plated holes, a switch's centre post and pegs, as keepouts of its image; traxide now reads them, placed and mirrored with their footprint.

The time is spent in a handful of long searches, as on the matrices: global routing corridors ([[TRX-26]]) and parallel batches ([[TRX-27]]) come next. The comparison with Freerouting 2.5 on this board runs in CI ([[TRX-15]]), where Freerouting can be downloaded.

## Octilinear estimate and weighted search (M1)

October 2026: the search estimates the remaining cost by octilinear instead of straight distance, and ranks states by that estimate weighted with the profile's `heuristic_weight` ([[Profiles]]).

Of matrix-6x17's 307 searches, 288 took under a millisecond; all the time went to 19 long column-to-header connections of 50–145 mm, each searching up to 15 million states. Measured on that board:

| Search | Time | Track length | Vias |
| --- | --- | --- | --- |
| Straight-distance estimate (before) | 33 s | 5942.6 mm | 35 |
| Octilinear estimate | 16.3 s | 5944.0 mm | 35 |
| Octilinear, weight 1.2 (the default) | **2.2 s** | 5962.6 mm (+0.3 %) | **19** |
| Octilinear, weight 1.5 | 0.74 s | 5977.6 mm (+0.6 %) | 52 |

| Board | Connections | Routed | Vias | Time | KiCad DRC |
| --- | --- | --- | --- | --- | --- |
| placements | 7 | 7 | 0 | < 0.01 s | clean |
| crossing | 2 | 2 | 2 | 0.07 s | clean |
| matrix-3x4 | 36 | 36 | 4 | 0.35 s | clean |
| matrix-6x17 | 306 | **306** | 24 | **2.7 s** | clean |

KiCad's DRC also caught a rule the DSN does not carry: KiCad keeps copper 0.5 mm from the board edge by default. Boards from KiCad now keep at least that distance (see [[Research/KiCad-integration]]).

## Rip-up and reroute (M1, TRX-23)

October 2026, the same search, plus negotiated rip-up and reroute and octilinear shortcuts ([[TRX-47]]).

| Board | Connections | Routed | Vias | Time | KiCad DRC |
| --- | --- | --- | --- | --- | --- |
| placements | 7 | 7 | 0 | < 0.01 s | clean |
| crossing | 2 | 2 | 2 | 0.07 s | clean |
| matrix-3x4 | 36 | 36 | 4 | 0.7 s | clean |
| matrix-6x17 | 306 | **306** | 35 | 33 s | **clean, nothing unconnected** |

Complete, and still slow: the next work is speed ([[TRX-24]] to [[TRX-27]], [[TRX-48]]).

## Walking skeleton (M0)

October 2026, sequential A* on a 0.1 mm octilinear grid, no rip-up, no global routing.

| Board | Connections | Routed | Vias | Time | KiCad DRC |
| --- | --- | --- | --- | --- | --- |
| placements (parts on both sides at 0°, 30°, 45°, 90°) | 7 | 7 | 0 | < 0.01 s | clean |
| crossing (two nets that must cross) | 2 | 2 | 2 | 0.15 s | clean |
| matrix-3x4 (12 keys: MX switches, diodes on the back, header) | 36 | 36 | 4 | 0.8 s | clean |
| matrix-6x17 (102 keys, a tenkeyless-sized matrix) | 306 | 304 | 30 | 34 s | 0 violations, the 2 unrouted connections |

![matrix-3x4 as routed by the walking skeleton and imported into KiCad 10: top copper red, bottom copper blue](images/matrix-3x4-m0.png)

The picture shows matrix-3x4 after the round trip: top copper red, bottom blue, vias as small rings. Columns run on top, rows on the bottom, and the header's column pins reach their columns through vias. The straight diagonals from the header and the stubs from switches to diodes run at any angle, which [[TRX-47]] changes to 45° steps.

### What the numbers say (M0)

- **Correct before fast.** KiCad agrees with traxide's own verification on every board: what traxide stores breaks no rule.
- **Most connections are trivial.** In matrix-3x4, 32 of 36 connections are straight lines; the rest cost the time. Four searches popped up to 470 000 states each, because the search state includes the heading (9 per node and layer) and a row of switch pads forces long detours that the straight-line estimate does not foresee.
- **Long connections are the problem.** The two failures in matrix-6x17 run from the outer columns to the header across the whole board, after everything else blocked their way. Rip-up and reroute ([[TRX-23]]) and global routing corridors ([[TRX-26]]) address exactly this; batches on all cores ([[TRX-27]]) multiply the rest.
- Already done in M0: dense search arrays reused between connections (3.5 s → 0.9 s on matrix-3x4), a fast board-edge test for rectangular outlines.
