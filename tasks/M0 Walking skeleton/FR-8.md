---
id: FR-8
title: Write Specctra SES in Board Access
type: feature
status: todo
priority: high
labels: [board-access, dsn]
estimate: 2
layer: access
---

`store_routes("board.ses", routes)`: wires and vias in a session file that KiCad imports.

- [ ] Session header, placement echo, `routes` with `network_out`, wires per layer and vias by padstack
- [ ] Round trip test: route nothing, write SES, import into KiCad with `kicad-cli` or by hand once, check

**Depends on:** [[FR-7]]
