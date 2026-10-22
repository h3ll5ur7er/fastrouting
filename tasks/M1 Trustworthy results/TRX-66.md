---
id: TRX-66
title: Keep the corpus history and comment regressions on pull requests
type: feature
status: backlog
priority: medium
labels: [benchmark, ci]
layer: manager
---

What [[TRX-22]] left: its gate compares each run with `corpus/baseline.json` and fails on a regression, but keeps no history across commits and says nothing on the pull request itself.

- [ ] `traxide bench corpus/ --profile …` (UC6 of [[Architecture/Use-cases#uc6-run-the-benchmark-corpus]], the Inspection Manager queuing route jobs) and `cargo xtask bench`
- [ ] Table per board: time, completion, vias, length, violations, against Freerouting and the previous release instead of a stored file
- [ ] CI job on main that stores the history and comments regressions on pull requests (needs a token that may write comments)
- [ ] Run the Benchmark workflow on main and on pull requests only, not on every push that touches the code

**Depends on:** [[TRX-21]] (a place to keep runs)
