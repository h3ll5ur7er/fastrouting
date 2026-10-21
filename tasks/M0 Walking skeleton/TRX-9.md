---
id: TRX-9
title: Load profiles in Profile Access
type: feature
status: done
priority: medium
labels: [profile]
estimate: 1.5
layer: access
---

`load_profile(name, overrides)` from built-in profiles (`fast`, `balanced`, `quality`), an optional `traxide.toml` and command line overrides ([[Architecture/Volatilities]] V13).

- [x] Built-in profiles as TOML compiled into the binary; only `default` for now, more when the router has trade-offs worth naming
- [x] Precedence: flags over `traxide.toml` over the built-in profile
- [x] Unknown keys are errors with a suggestion

**Depends on:** [[TRX-4]]
