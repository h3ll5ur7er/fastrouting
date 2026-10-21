---
id: TRX-51
title: Write DRC reports in KiCad's JSON format
type: feature
status: done
priority: low
labels: [cli, compatibility, verification]
estimate: 1
layer: access
---

Freerouting's `-drc report.json` writes a DRC report in KiCad's JSON schema; `traxide freerouting` warned that it didn't yet ([[TRX-34]]). Now traxide writes its own verdict that way: Board Access holds the format, as it holds KiCad's other formats, and the managers store the report when a job names a file. How traxide's verdict maps onto KiCad's format, and what it leaves out: [[Research/KiCad-integration]], section *DRC reports*.

- [x] A report of traxide's own verdict in KiCad's DRC JSON schema: violations and unconnected items with their positions and descriptions
- [x] `-drc` in `traxide freerouting`, and `--drc` in `traxide route`
- [x] `--drc` in `traxide check`
- [x] Checked against a report `kicad-cli pcb drc` writes for the same routed board: on the routed `crossing`, `matrix-3x4` and `keyboard-tkl` boards both are clean and agree in every field but `date`, `kicad_version` and KiCad's optional `included_severities` and `ignored_checks`. Violations are checked against the shape of KiCad 10's entries; no KiCad was at hand to judge a broken board alongside

**Depends on:** [[TRX-34]]
