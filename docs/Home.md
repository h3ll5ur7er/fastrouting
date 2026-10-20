# fastrouting

A fast, dependency-free PCB autorouter for KiCad and every tool that speaks Specctra DSN: a modern alternative to Freerouting.

Freerouting does the job, but it has needed a Java runtime until its native builds of October 2026, still routes on one CPU core, and can take hours on boards like a 4-layer RP2350 tenkeyless keyboard ([Prior art](Research/Prior-art.md)). fastrouting is one native binary per platform, uses every CPU core and, when there is one, the GPU, and routes with the algorithms that replaced maze routing in chip design: negotiated congestion global routing followed by parallel detailed routing.

> [!NOTE]
> fastrouting is at the design stage. This wiki holds the research and the architecture; the task board holds the plan.

## Goals

- **Fast**: route a 4-layer keyboard PCB (~90 switches and diodes, an RP2350, USB) in seconds, not hours. Track it in the benchmark corpus.
- **No dependencies**: one static binary for Windows, macOS and Linux on x86-64 and ARM64. No JVM, no Python, no GPU SDK. A GPU helps when present and is never required.
- **Correct and better**: every result passes an independent DRC, and releases are gated on completion and DRC-clean rates on the public PCBench and PCBWorld corpora, against Freerouting.
- **Deterministic**: the same input gives the same output on any number of threads, CPU or GPU.
- **Drop-in**: reads DSN and writes SES like Freerouting, accepts Freerouting's command line, and integrates with KiCad directly through its IPC API.

## Pages

| Page | Covers |
| --- | --- |
| [Research](Research.md) | State-of-the-art routing algorithms, prior art, KiCad integration and the tech stack, with sources |
| [Architecture](Architecture.md) | The iDesign decomposition: [volatilities](Architecture/Volatilities.md), components, [use cases](Architecture/Use-cases.md), [contracts](Architecture/Contracts.md), the [routing pipeline](Architecture/Routing-pipeline.md), [parallelism and GPU](Architecture/Parallelism-and-GPU.md) |
| [Project plan](Project-plan.md) | Milestones, the dependency network of the tasks and the critical path |
| [Development](Development.md) | Repository layout, layer rules, building, testing and benchmarking |
| [Glossary](Glossary.md) | The routing and design terms used here |
