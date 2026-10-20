# Benchmarks

Measurements of traxide on the end-to-end boards, by hand until the benchmark of [[TRX-22]] records them in CI. Every routed board below was imported into KiCad 10.0.7 and passed its DRC with no violations.

[TOC]

## How to measure

```bash
cargo xtask e2e placements crossing matrix-3x4 matrix-6x17   # builds, routes and checks the recipes
time target/release/traxide route -q target/e2e/matrix-6x17/matrix-6x17.dsn
```

The machine: a 4-core cloud container (x86-64), one thread used.

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
