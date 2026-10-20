# Project plan

The plan follows the project design half of the iDesign Method: one activity per component, built in the order of the dependencies between them, with integration points where a use case first runs end to end. The activities are the tasks on the board; each has an `estimate` in ideal developer days, the iDesign `layer` it builds and the tasks it depends on.

[TOC]

## Milestones

Epics are milestones: `tasks/<milestone>/FR-n.md`.

| Milestone | Integration point: done when | Effort (days) |
| --- | --- | --- |
| **M0 Walking skeleton** | A simple 2-layer board routes from DSN to SES on the command line and imports into KiCad (UC1, sequential) | 38.5 |
| **Research spikes** | The corpus and Freerouting baselines exist; wgpu and the KiCad IPC API are proven or ruled out | 7.5 |
| **M1 Trustworthy results** | Every result is verified independently; the corpus runs in CI against Freerouting; the keyboard routes completely (UC5, UC6) | 20 |
| **M2 Parallel routing** | Global routing plus parallel detailed routing on every core, with identical results on any thread count | 18 |
| **M3 GPU acceleration** | Global routing kernels on the GPU where the benchmark shows a gain, identical results to the CPU | 13 |
| **M4 KiCad integration** | Route from `.kicad_pcb` headless and inside KiCad with undo; Freerouting's CLI accepted; release binaries for six targets (UC2, UC3) | 20.5 |
| **M5 Routing quality** | Optimisation, push and shove, differential pairs, length matching; corpus completion and DRC-clean rate above Freerouting's (UC4) | 28 |
| Later | Viewer, web demo, BGA escape, HTTP API (UC7) | 17 |

Total: about **165 developer days** including [[FR-1]].

## Dependency network

The critical path is drawn bold red.

```mermaid
flowchart LR
    FR1["FR-1 Architecture"]
  subgraph Research["Research spikes"]
    FR15["FR-15 Corpus, baselines"]
    FR16["FR-16 Spike: wgpu"]
    FR17["FR-17 Spike: KiCad IPC"]
    FR18["FR-18 Spike: Rust routers"]
  end
  subgraph M0["M0 Walking skeleton"]
    FR2["FR-2 Workspace"]
    FR3["FR-3 Layer check, CI"]
    FR4["FR-4 Contracts"]
    FR5["FR-5 Geometry"]
    FR6["FR-6 Diag, events, pool"]
    FR7["FR-7 DSN reader"]
    FR8["FR-8 SES writer"]
    FR9["FR-9 Profile Access"]
    FR10["FR-10 Rules: basic"]
    FR11["FR-11 Layout Access"]
    FR12["FR-12 Detailed: A*"]
    FR13["FR-13 Routing Manager"]
    FR14["FR-14 CLI route"]
  end
  subgraph M1["M1 Trustworthy results"]
    FR19["FR-19 Verification"]
    FR20["FR-20 Check use case"]
    FR21["FR-21 Run Access"]
    FR22["FR-22 Benchmark gate"]
    FR23["FR-23 Rip-up, gridless"]
  end
  subgraph M2["M2 Parallel routing"]
    FR24["FR-24 Planning"]
    FR25["FR-25 Compute: CPU"]
    FR26["FR-26 Global routing"]
    FR27["FR-27 Parallel detailed"]
  end
  subgraph M3["M3 GPU acceleration"]
    FR28["FR-28 Compute: GPU"]
    FR29["FR-29 GPU global"]
  end
  subgraph M4["M4 KiCad integration"]
    FR30["FR-30 KiCad files"]
    FR31["FR-31 KiCad IPC"]
    FR32["FR-32 Rules: KiCad"]
    FR33["FR-33 KiCad plugin"]
    FR34["FR-34 Freerouting CLI"]
    FR35["FR-35 Release"]
  end
  subgraph M5["M5 Routing quality"]
    FR36["FR-36 Optimization"]
    FR37["FR-37 Push and shove"]
    FR38["FR-38 Diff pairs"]
    FR39["FR-39 Length matching"]
    FR40["FR-40 Checkpoints"]
  end
  subgraph Later["Later"]
    FR41["FR-41 Viewer"]
    FR42["FR-42 Web demo"]
    FR43["FR-43 BGA escape"]
    FR44["FR-44 HTTP API"]
  end
  FR1 --> FR2
  FR2 --> FR3
  FR2 --> FR4
  FR2 --> FR5
  FR2 --> FR6
  FR4 --> FR7
  FR5 --> FR7
  FR7 --> FR8
  FR4 --> FR9
  FR4 --> FR10
  FR4 --> FR11
  FR5 --> FR11
  FR10 --> FR11
  FR11 --> FR12
  FR7 --> FR13
  FR8 --> FR13
  FR9 --> FR13
  FR10 --> FR13
  FR11 --> FR13
  FR12 --> FR13
  FR6 --> FR13
  FR13 --> FR14
  FR1 --> FR15
  FR1 --> FR16
  FR1 --> FR17
  FR1 --> FR18
  FR10 --> FR19
  FR11 --> FR19
  FR19 --> FR20
  FR21 --> FR20
  FR4 --> FR21
  FR15 --> FR22
  FR20 --> FR22
  FR21 --> FR22
  FR14 --> FR22
  FR12 --> FR23
  FR19 --> FR23
  FR13 --> FR24
  FR4 --> FR25
  FR6 --> FR25
  FR24 --> FR26
  FR25 --> FR26
  FR23 --> FR27
  FR26 --> FR27
  FR25 --> FR28
  FR16 --> FR28
  FR26 --> FR29
  FR28 --> FR29
  FR7 --> FR30
  FR30 --> FR31
  FR17 --> FR31
  FR10 --> FR32
  FR30 --> FR32
  FR31 --> FR33
  FR13 --> FR33
  FR14 --> FR34
  FR3 --> FR35
  FR14 --> FR35
  FR27 --> FR36
  FR19 --> FR36
  FR27 --> FR37
  FR27 --> FR38
  FR32 --> FR38
  FR36 --> FR39
  FR38 --> FR39
  FR27 --> FR40
  FR21 --> FR40
  FR13 --> FR41
  FR27 --> FR42
  FR41 --> FR42
  FR24 --> FR43
  FR13 --> FR44
  classDef critical stroke-width:3px,stroke:#d33
  class FR1,FR2,FR4,FR10,FR11,FR12,FR13,FR24,FR26,FR27,FR36,FR39 critical
```

## Critical path

With enough developers, the longest chain decides the end date:

[[FR-1]] → [[FR-2]] → [[FR-4]] → [[FR-10]] → [[FR-11]] → [[FR-12]] → [[FR-13]] → [[FR-24]] → [[FR-26]] → [[FR-27]] → [[FR-36]] → [[FR-39]]: **49 days**.

- Contracts, rules, Layout Access and the first detailed router are the spine: everything waits for them. Put the strongest developer on the critical path and keep it free of other work.
- M4 (KiCad) hangs off the DSN reader, not off the router, so a second developer can start it around day 7: plugin and files are ready by day ~27.
- The research spikes need only [[FR-1]] and can start at once; the benchmark corpus ([[FR-15]]) must be ready before M1 closes.

## Staffing

| Team | Duration (estimate) | Assignment |
| --- | --- | --- |
| 1 developer | ~165 days | Milestones in order, spikes first |
| 2 developers | ~85 days | A: critical path (contracts, rules, layout, engines). B: utilities, Board Access, spikes, CLI, then M4 |
| 3 developers | ~60 days | As two, plus C: Verification, Run Access, benchmark (M1), then GPU (M3) |

Estimates are ideal days; add the usual margin for integration and review. Re-plan after M0, once the walking skeleton has measured the real cost of the spine.
