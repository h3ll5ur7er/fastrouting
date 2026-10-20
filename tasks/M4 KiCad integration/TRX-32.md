---
id: TRX-32
title: Compile KiCad net classes and custom rules in the Rules Engine
type: feature
status: todo
priority: high
labels: [rules, kicad]
estimate: 4
layer: engine
---

- [ ] Net class rules from `.kicad_pro`
- [ ] `.kicad_dru`: rule conditions (`A.NetClass == 'HV'`, layers, `A.inDiffPair`…) through a small evaluator, compiled into the constraint tables
- [ ] Compare with KiCad's DRC on the corpus: no rule we route against may disagree

**Depends on:** [[TRX-10]], [[TRX-30]]
