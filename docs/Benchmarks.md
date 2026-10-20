# Benchmarks

Measurements of traxide on the end-to-end boards, by hand until the benchmark of [[TRX-22]] records them in CI. Every routed board below was imported into KiCad 10.0.7 and passed its DRC with no violations.

[TOC]

## How to measure

```bash
cargo xtask e2e placements crossing matrix-3x4 matrix-6x17   # builds, routes and checks the recipes
time target/release/traxide route -q target/e2e/matrix-6x17/matrix-6x17.dsn
```

The machine: a 4-core cloud container (x86-64), one thread used.

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

### What the numbers say

- **Correct before fast.** KiCad agrees with traxide's own verification on every board: what traxide stores breaks no rule.
- **Most connections are trivial.** In matrix-3x4, 32 of 36 connections are straight lines; the rest cost the time. Four searches popped up to 470 000 states each, because the search state includes the heading (9 per node and layer) and a row of switch pads forces long detours that the straight-line estimate does not foresee.
- **Long connections are the problem.** The two failures in matrix-6x17 run from the outer columns to the header across the whole board, after everything else blocked their way. Rip-up and reroute ([[TRX-23]]) and global routing corridors ([[TRX-26]]) address exactly this; batches on all cores ([[TRX-27]]) multiply the rest.
- Already done in M0: dense search arrays reused between connections (3.5 s → 0.9 s on matrix-3x4), a fast board-edge test for rectangular outlines.
