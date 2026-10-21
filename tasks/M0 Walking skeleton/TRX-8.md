---
id: TRX-8
title: Write Specctra SES in Board Access
type: feature
status: done
priority: high
labels: [board-access, dsn]
estimate: 2
layer: access
---

`store_routes("board.ses", routes)`: wires and vias in a session file that KiCad imports.

- [x] Session header, placement echo, `routes` with `network_out`, wires per layer and vias by padstack
- [x] Round trip test; KiCad 10 importing it is part of [[TRX-45]]

**Depends on:** [[TRX-7]]
