---
id: FR-9
title: Load profiles in Profile Access
type: feature
status: todo
priority: medium
labels: [profile]
estimate: 1.5
layer: access
---

`load_profile(name, overrides)` from built-in profiles (`fast`, `balanced`, `quality`), an optional `fastrouting.toml` and command line overrides ([[Architecture/Volatilities]] V13).

- [ ] Built-in profiles as TOML compiled into the binary
- [ ] Precedence: flags over `fastrouting.toml` over the built-in profile
- [ ] Unknown keys are errors with a suggestion

**Depends on:** [[FR-4]]
