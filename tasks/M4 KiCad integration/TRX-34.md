---
id: TRX-34
title: Accept Freerouting's command line
type: feature
status: done
priority: medium
labels: [cli, compatibility]
estimate: 1.5
layer: client
---

A drop-in for tools that call Freerouting ([[Research/Prior-art#interfaces-worth-matching]]).

- [x] `-de` (the `.dsn` of `+`-joined files), `-do` (sessions only), `-mt` and `-inc` mapped onto the route job; unknown net classes are errors naming the known ones
- [x] Options that tune Freerouting's algorithm (`-mp`, `-us`, `-hr`, `-is`, `-im`, `-oit`, `--router.*`), `-dr` and `-drc` warn; GUI, logging, language and analytics options are ignored; unknown options warn
- [x] Invoked as `traxide freerouting …` or through a `freerouting` binary name; exits 0 whenever it wrote a session, as tools that run Freerouting import it then

`-drc` reports follow in [[TRX-51]]. Implemented from Freerouting's documentation of its arguments (`docs/command_line_arguments.md` in its repository), never its code.

**Depends on:** [[TRX-14]]
