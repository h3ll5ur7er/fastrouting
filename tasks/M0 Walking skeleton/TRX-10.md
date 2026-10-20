---
id: TRX-10
title: Compile basic design rules in the Rules Engine
type: feature
status: done
priority: high
labels: [rules]
estimate: 3
layer: engine
---

Compile `RuleSource` into a `ConstraintSet` ([[Architecture/Routing-pipeline#compile-rules]]).

- [x] Clearance matrix by (net class, net class, layer), defaults and DSN rule scopes
- [x] Widths per net and layer; via padstacks allowed per net
- [x] O(1) lookups; unit tests for precedence

**Depends on:** [[TRX-4]]
