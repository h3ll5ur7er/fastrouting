---
id: FR-4
title: Define the data and service contracts
type: feature
status: todo
priority: high
labels: [contracts]
estimate: 3
layer: contract
---

Turn the sketch in [[Architecture/Contracts]] into the `contracts` crate.

- [ ] Ids as dense typed indices; `Design`, `Stackup`, `Net`, `Pad`, `Keepout`, `Route`, `RuleSource`
- [ ] `ConstraintSet`, `Plan`, `Connection`, `Batch`, `Corridors`, `Routes`, `Report`, `Job`, `Profile`, `Event`
- [ ] Traits for every manager, engine and resource access, generic over the access traits (no `dyn` in hot paths)
- [ ] Doc comments state units (nm) and invariants

Done when every component crate compiles against its trait with `todo!()` bodies.

**Depends on:** [[FR-2]]
