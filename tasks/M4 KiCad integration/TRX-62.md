---
id: TRX-62
title: Check custom rules against KiCad's DRC on real boards
type: improvement
status: backlog
priority: medium
labels: [rules, kicad]
layer: engine
---

Custom rules (`.kicad_dru`) are compiled as KiCad 10's DRC was observed to apply them on small hand-written boards ([[TRX-32]], [[Research/KiCad-integration#custom-rules]]). Real boards may use what those did not.

- [ ] Compare with KiCad's DRC on corpus boards with their own files: no rule traxide routes against may disagree (needs [[TRX-30]]'s corpus from board files)
- [ ] `memberOfFootprint`: TokenBoard ([[TRX-64]]) lowers the edge clearance to 0.2 mm at the cutouts of its LED footprints (`A.memberOfFootprint('LED*') || B.memberOfFootprint('LED*')`). traxide cannot tell, keeps the board's 0.3 mm there, and `traxide check` reports 4 edge violations on the author's routing that KiCad accepts ([[TRX-65]]). Cutouts would need to know their footprint, and the edge clearance to vary by cutout
- [ ] If boards need them: `disallow`, `physical_clearance`, `annular_width`, per-layer values, vias of the sizes rules ask
