# Benchmarks

Measurements of traxide on the end-to-end boards, by hand until the benchmark of [[TRX-22]] records them in CI. Every routed board below was imported into KiCad 10.0.7 and passed its DRC with no violations.

[TOC]

## How to measure

```bash
cargo xtask e2e placements crossing matrix-3x4 matrix-6x17   # builds, routes and checks the recipes
time target/release/traxide route -q target/e2e/matrix-6x17/matrix-6x17.dsn
```

The machine: a 4-core cloud container (x86-64), one thread used.

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
