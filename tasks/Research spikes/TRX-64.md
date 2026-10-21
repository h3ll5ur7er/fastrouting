---
id: TRX-64
title: Add KiCad projects to the corpus, starting with TokenBoard
type: improvement
status: review
priority: high
assignee: claude-corpus
labels: [benchmark, kicad]
layer: infrastructure
---

The corpus ([[xtask/src/corpus.rs]], [[corpus/freerouting-fixtures.txt]]) holds only DSN files from Freerouting's bug reports. Real KiCad projects test what DSN loses (custom rules, zones, KiCad's own power layers) and are what users route. The first: the maintainer's own keyboard, [TokenBoard](https://github.com/h3ll5ur7er/tokenboard) (`kb/kb.kicad_pcb` with `kb.kicad_pro` and `kb.kicad_dru`; KiCad 10, 4 layers with In1.Cu and In2.Cu as power planes, 420 footprints, 7 zones), routed by Freerouting and committed routed. Public; the maintainer owns it and asked for it to be added.

Tried by hand on commit 478b4f9 (tracks, arcs and vias stripped, zones kept):

| | Nets complete | Pads unconnected | Vias | Time |
| --- | --- | --- | --- | --- |
| Freerouting, as committed | 238 of 241 | 438 (the power nets, which its zones and planes join) | 638 | |
| traxide, 331073f | 228 of 241 | 39 | 1179 | 355 s |

Both clean by `traxide check`; traxide routes the power nets with tracks, as it cannot join pads through zones yet ([[TRX-58]]).

**How it works now.** [[corpus/kicad-projects.txt]] names each board by repository, commit and path; [[xtask/src/kicad_project.rs]] parses those lines and strips a board's unlocked tracks, arcs and vias at the text level, leaving every other byte as it was. [[xtask/src/corpus.rs]] downloads the board with its `.kicad_pro` and `.kicad_dru` into `target/corpus/projects/<name>-<commit>/`, routes the stripped board with traxide, has KiCad (Docker image `kicad/kicad:10.0`, or `--local`) export its DSN for Freerouting (`roundtrip.py dsn`) and import Freerouting's session (`roundtrip.py import`), and judges the stripped board, traxide's, Freerouting's and the committed one with `traxide check`. The table gains a column "Routed by its author" when a project is in it.

**Decisions.**
- TokenBoard keeps the 634 tracks and 439 vias its author locked: the hand-routed part of the board, which the first trial above had stripped too.
- traxide routes projects with `--time-limit` at three quarters of `--timeout` (90 s of 120 s in CI): unlimited it takes 189 s on 4 cores, and a board stopped at the limit gives the gate nothing to compare. Its result then moves by a few nets between runs (236 and 239 of 241 locally), within the gate's 2 %.
- Judged by `traxide check`, not KiCad's DRC, like the rest of the corpus: KiCad's DRC would mark TokenBoard as breaking rules ([[TRX-65]]) and the gate fails on any broken rule.

Measured on commit 22f4462, 4 cores, 120 s limit (the baseline): traxide 90.8 s, 236 of 241 nets, 0 rules broken; Freerouting stopped at 120 s; the board as committed 238 of 241 nets, 0 rules broken. Details in [[Benchmarks#a-real-kicad-project-tokenboard-trx-64]]. The planes work ([[TRX-58]]) changes these numbers: the main session measured traxide at 236 of 241 nets with 5 pads unconnected in 154 s, and the author's board at 240 of 241 nets with 27 pads unconnected, as plane joins count. The baseline is raised once both are merged.

- [x] Corpus entries that name a KiCad project by URL at a fixed commit: the board with its `.kicad_pro` and `.kicad_dru` downloaded beside it, as for DSN never committed here
- [x] A routed board is stripped of its unlocked tracks, arcs and vias before routing (zones and locked copper stay), and the board as committed is judged as the baseline it is ("routed by its author")
- [x] Freerouting runs on a DSN exported by KiCad (in its Docker image, as `cargo xtask e2e` exports), or is left out for such boards, said in the table
- [x] TokenBoard in the corpus, pinned to a commit, and its row in [[Benchmarks]]
- [x] Say in [[Development#benchmarking]] how to add a board: what the entry looks like, and that the board's licence must allow it (the corpus downloads, never stores)
- [x] TokenBoard in the gate's baseline ([[corpus/baseline.json]]), with traxide's 120 s result: 232 of 241 nets in 92.9 s with [[TRX-65]] merged, the edge clearance from the LED cutouts kept (236 before, with 199 edge clearance violations that KiCad found and `traxide check` did not)
- [x] traxide stops negotiating on a project at three quarters of `--timeout`, so that a board too big for the limit finishes and the gate compares routes
- [ ] The Benchmark workflow green with TokenBoard in shard 4 (its first CI run: the runner's KiCad export and Freerouting's session import)
- [ ] Judge KiCad projects with KiCad's DRC as well, zones refilled: with [[TRX-65]] merged it finds traxide's TokenBoard clean, but `traxide check` counts 4 edge clearance violations on the author's board that KiCad does not
- [x] Freerouting's result on TokenBoard: no session after 28 min, nor after 40 min with `-mp 2` (still in its fanout stage); in CI it stops at the corpus's 120 s. The way its session takes (KiCad's DSN export, KiCad's import) was tried with traxide's Freerouting mode in its place: it works, and shows the DSN losing TokenBoard's 0.2 mm clearances of VLED and GND (38 broken)
