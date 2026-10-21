---
id: TRX-32
title: Compile KiCad net classes and custom rules in the Rules Engine
type: feature
status: done
priority: high
labels: [rules, kicad]
estimate: 4
layer: engine
---

- [x] Net class rules from `.kicad_pro` (with [[TRX-30]])
- [x] `.kicad_dru` read by Board Access into `DesignRules.custom`: `clearance`, `track_width`, `via_diameter`, `hole_size`, `edge_clearance`, `hole_clearance`; conditions on `NetClass` and `NetName` with `&&`, `||`, `!`; layers `outer`, `inner` and by name
- [x] Rules Engine: the last rule that applies wins; tests and layers it cannot tell only ever tighten ([[Research/KiCad-integration#custom-rules]])
- [x] End-to-end: recipe `isolation` with a `.kicad_dru`, routed from its board file, clean in KiCad 10's DRC
- [x] A broken `.kicad_dru` warns and routes with what can be read, as the maintainer decided (October 2026): a rule traxide cannot read is left out with its name and line, a file without `(version 1)` is read anyway, an unreadable file is left out whole ([[Research/KiCad-integration#custom-rules]])
Moved to [[TRX-62]]: comparing with KiCad's DRC on real boards, and the constraints not compiled yet.

**Depends on:** [[TRX-10]], [[TRX-30]]
