---
id: TRX-4
title: Define the data and service contracts
type: feature
status: done
priority: high
labels: [contracts]
estimate: 3
layer: contract
---

Turn the sketch in [[Architecture/Contracts]] into the `contracts` crate.

- [x] Ids as dense typed indices; `Design`, `Stackup`, `Net`, `Pad`, `Keepout`, `Route`, `RuleSource`
- [x] `ConstraintSet`, `Plan`, `Connection`, `Batch`, `Corridors`, `Routes`, `Report`, `Job`, `Profile`, `Event`
- [x] `LayoutAccess` as the trait engines use; plain functions where a trait would have one implementation and only managers as callers (see [[Architecture/Contracts#service-contracts]])
- [x] Doc comments state units (nm) and invariants

Built as needed by the walking skeleton: `Batch` and `Corridors` arrive with [[TRX-24]] and [[TRX-26]], run reports with [[TRX-21]].

**Depends on:** [[TRX-2]]
