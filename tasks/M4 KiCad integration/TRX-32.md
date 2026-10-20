---
id: TRX-32
title: Compile KiCad net classes and custom rules in the Rules Engine
type: feature
status: review
priority: high
labels: [rules, kicad]
estimate: 4
layer: engine
---

- [x] Net class rules from `.kicad_pro` (with [[TRX-30]])
- [x] `.kicad_dru` read by Board Access into `DesignRules.custom`: `clearance`, `track_width`, `via_diameter`, `hole_size`, `edge_clearance`, `hole_clearance`; conditions on `NetClass` and `NetName` with `&&`, `||`, `!`; layers `outer`, `inner` and by name
- [x] Rules Engine: the last rule that applies wins; tests and layers it cannot tell only ever tighten ([[Research/KiCad-integration#custom-rules]])
- [x] End-to-end: recipe `isolation` with a `.kicad_dru`, routed from its board file, clean in KiCad 10's DRC
- [ ] Compare with KiCad's DRC on the corpus: no rule we route against may disagree (needs corpus boards with their own files, [[TRX-30]])
- [ ] Later, if boards need them: `disallow`, `physical_clearance`, `annular_width`, per-layer values, vias of the sizes rules ask

**Depends on:** [[TRX-10]], [[TRX-30]]
