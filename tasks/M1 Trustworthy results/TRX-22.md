---
id: TRX-22
title: Run the benchmark corpus and gate releases on it
type: feature
status: todo
priority: high
labels: [benchmark, ci]
estimate: 3
layer: manager
---

UC6 of [[Architecture/Use-cases#uc6-run-the-benchmark-corpus]]: the Inspection Manager queues route jobs and compares.

- [ ] `traxide bench corpus/ --profile …` and `cargo xtask bench`
- [ ] Table per board: time, completion, vias, length, violations, against Freerouting and the previous release
- [ ] CI job on main that stores the history and comments regressions on pull requests

**Depends on:** [[TRX-15]], [[TRX-20]], [[TRX-21]], [[TRX-14]]
