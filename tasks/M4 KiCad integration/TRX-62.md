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
- [ ] If boards need them: `disallow`, `physical_clearance`, `annular_width`, per-layer values, vias of the sizes rules ask
