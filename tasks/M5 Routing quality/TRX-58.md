---
id: TRX-58
title: Join pads through their net's planes
type: feature
status: in-progress
priority: medium
assignee: claude-planes
labels: [board-access, planning]
estimate: 3
layer: engine
---

A DSN states copper planes as `(plane NET (polygon LAYER …))`: a pour that joins every pad and via of its net that it touches. traxide does not read them ([[crates/access/board/src/specctra/dsn.rs]]), so 19 corpus boards lose their planes ([[Corpus-analysis]]). The connections of a plane's net are then routed as tracks across the board, take room from the other nets, and fail where a via down to the plane would do: on Issue269-z10_module (GND on In1.Cu, +3V3 on In2.Cu, both planes of type power) the GND connections R103-2 → IC101-28 and IC101-28 → IC101-42 cannot be routed at all, while a via next to each pad reaches the plane.

- [ ] Board Access reads `(plane …)` into the design: net, layer, polygon
- [ ] Planning: pads a plane touches, through-hole pads on the plane's layer and the plane itself are joined; an SMD pad of the net on another layer needs only a short connection to a via onto the plane, not a track to the next pad
- [ ] Layout and verification: the plane's copper, less the clearance around other nets' copper as KiCad refills a zone, is fixed copper of its net; other nets keep their clearance from it on its layer
- [ ] Tests: a 4-layer board with a GND plane on an inner layer joins its GND pads by vias; Issue269-z10_module and Issue093-interf_u (GND, which a bottom pour joins) before → after
- [ ] Measure the boards with planes in the corpus (Issue015, 022, 027, 029, 054, 093, 113, 145, 153, 163, 208, 214, 219, 230 ×2, 269 ×3, 367) for time and completion before → after
