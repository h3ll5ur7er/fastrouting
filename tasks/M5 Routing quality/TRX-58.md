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

- [x] Board Access reads `(plane …)` into the design: net, layer, polygon; and a `.kicad_pcb`'s copper zones, one plane per layer (the IPC adapter reads the same board text)
- [x] KiCad boards: a layer of type power takes tracks unless a copper zone fills it, as [[TRX-57]] did for DSN
- [x] Planning: pads a plane touches, through-hole pads on the plane's layer and the plane itself are joined; an SMD pad of the net on another layer needs only a short connection to a via onto the plane, not a track to the next pad ([[crates/engines/planning/src/planes.rs]]: the nearest place within 2 mm of the pad's edge, on a 0.05 mm grid, where a via onto the plane and a track to it fit, placed before anything is routed and held as the board's own copper)
- [ ] Layout and verification: the plane's copper, less the clearance around other nets' copper as KiCad refills a zone, is fixed copper of its net; other nets keep their clearance from it on its layer
- [x] Verification: copper of a plane's net on the plane, where no other net's plane overlaps it, is joined ([[crates/engines/verification/src/pieces.rs]])
- [ ] Tests: a 4-layer board with a GND plane on an inner layer joins its GND pads by vias; Issue269-z10_module and Issue093-interf_u (GND, which a bottom pour joins) before → after
- [ ] Measure the boards with planes in the corpus (Issue015, 022, 027, 029, 054, 093, 113, 145, 153, 163, 208, 214, 219, 230 ×2, 269 ×3, 367) for time and completion before → after

## Decisions

- **Only planes on layers that take no tracks join.** Most planes of the corpus are pours over the whole of both outer layers of a 2-layer board (Issue015, 022, 027, 029, 054, 113, 208, 367): KiCad refills them around the routes, so tracks of other nets may cut them apart anywhere, and only refilling tells which pads they still join. Treating such pours as fixed copper would leave those boards nothing to route on. A layer of type power that a plane fills takes no tracks ([[TRX-57]]), only vias, whose antipads rarely cut a plane apart: there a plane is counted on. Pours on layers that take tracks are routed as before (Issue093's bottom pour too); the route then only gains copper from the pour.
- **A plane yields to the copper of other nets.** As KiCad fills a zone less a clearance around the copper of other nets, nothing of another net conflicts with a plane: Layout Access keeps no obstacle for it, and vias of other nets pass through it as through any power layer.
- **Overlapping planes of different nets join nothing where they overlap**: which fills there is their priority's choice, which DSN does not carry (Issue269-z10_module: a plane of Net-(D201-K) inside the GND and +3V3 planes).
- **Copper joins a plane only lying wholly inside its outline.** Copper reaching over its edge may still touch the fill; counting it as not joined only routes a connection more.
