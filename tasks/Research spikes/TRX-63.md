---
id: TRX-63
title: Add KiCad projects to the corpus, starting with TokenBoard
type: improvement
status: todo
priority: high
assignee: claude-gate
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

- [ ] Corpus entries that name a KiCad project by URL at a fixed commit: the board with its `.kicad_pro` and `.kicad_dru` downloaded beside it, as for DSN never committed here
- [ ] A routed board is stripped of its unlocked tracks, arcs and vias before routing (zones and locked copper stay), and the board as committed is judged as the baseline it is ("routed by its author")
- [ ] Freerouting runs on a DSN exported by KiCad (in its Docker image, as `cargo xtask e2e` exports), or is left out for such boards, said in the table
- [ ] TokenBoard in the corpus, pinned to a commit, and its row in [[Benchmarks]]
- [ ] Say in [[Development#benchmarking]] how to add a board: what the entry looks like, and that the board's licence must allow it (the corpus downloads, never stores)
