---
id: TRX-51
title: Write DRC reports in KiCad's JSON format
type: feature
status: backlog
priority: low
labels: [cli, compatibility, verification]
estimate: 1
layer: client
---

Freerouting's `-drc report.json` writes a DRC report in KiCad's JSON schema; `traxide freerouting` warns that it doesn't yet ([[TRX-34]]).

- [ ] A report of traxide's own verdict in KiCad's DRC JSON schema: violations and unconnected items with their positions and descriptions
- [ ] `-drc` in `traxide freerouting`, and `--drc` in `traxide route`
- [ ] Checked against a report `kicad-cli pcb drc` writes for the same routed board

**Depends on:** [[TRX-34]]
