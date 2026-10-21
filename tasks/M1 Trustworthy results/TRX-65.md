---
id: TRX-65
title: Keep the edge clearance from cutouts inside footprints
type: bug
status: todo
priority: high
labels: [rules, kicad, verification]
layer: access
---

Found by the corpus on TokenBoard ([[TRX-64]]): traxide routes GND tracks on B.Cu 0.136 mm from the cutouts that the reverse-mount LED footprints (SK6812MINI-E) draw on Edge.Cuts, and `traxide check` finds nothing wrong. KiCad 10's DRC, with the zones refilled (`kicad-cli pcb drc --refill-zones`), reports 199 `copper_edge_clearance` violations on the routed board: "Board edge clearance violation (rule 'LED cutout' clearance 0.2000 mm; actual 0.1359 mm)", between an arc of an LED footprint on Edge.Cuts and a track. The board's own edge clearance is 0.3 mm (`min_copper_edge_clearance` in `kb.kicad_pro`); its custom rule lowers it to 0.2 mm near the LEDs (`A.memberOfFootprint('LED*') || B.memberOfFootprint('LED*')` in `kb.kicad_dru`). The board as committed passes the same DRC with no violation.

A closed drawing inside the outline becomes a keepout ([[crates/access/board/src/kicad/board.rs]], `outline`), which seems to be kept at copper clearance, not edge clearance; arcs may also be cut by their chords. To reproduce: `cargo xtask corpus --list corpus/kicad-projects.txt`, then KiCad's DRC with `--refill-zones` on `target/corpus/projects/tokenboard-kb-478b4f9/traxide.kicad_pcb`.

- [ ] A failing test: a track beside a cutout drawn in a footprint, judged by the verification engine against the edge clearance and the custom rule that names the footprint
- [ ] Routing keeps that clearance from inner cutouts
- [ ] TokenBoard clean in KiCad's DRC with zones refilled
