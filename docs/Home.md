# traxide

*trace* + *oxide*: a fast, dependency-free PCB autorouter for KiCad and every tool that speaks Specctra DSN: a modern alternative to Freerouting.

Freerouting does the job, but it has needed a Java runtime until its native builds of October 2026, still routes on one CPU core, and can take hours on boards like a 4-layer RP2350 tenkeyless keyboard ([Prior art](Research/Prior-art.md)). traxide is one native binary per platform, uses every CPU core and, when there is one, the GPU, and routes with the algorithms that replaced maze routing in chip design: negotiated congestion global routing followed by parallel detailed routing.

> [!NOTE]
> traxide routes a 4-layer RP2350 ISO TKL keyboard completely in 1.5 s on a CI runner, clean in KiCad 10's DRC, where Freerouting 2.5 takes 105–481 s and leaves 199 tracks narrower than the board allows ([[Benchmarks]]). It reads DSN and writes SES; reading KiCad's own files and a KiCad plugin come next ([Project plan](Project-plan.md)).

## Goals

- **Fast**: route a 4-layer keyboard PCB (~90 switches and diodes, an RP2350, USB) in seconds, not hours. Track it in the benchmark corpus.
- **No dependencies**: one static binary for Windows, macOS and Linux on x86-64 and ARM64. No JVM, no Python, no GPU SDK. A GPU helps when present and is never required.
- **Correct and better**: every result passes an independent DRC, and releases are gated on completion and DRC-clean rates on the public PCBench and PCBWorld corpora, against Freerouting.
- **Deterministic**: the same input gives the same output on any number of threads, CPU or GPU.
- **Permissive**: `MIT OR Apache-2.0`, written clean-room, so it is usable where copyleft is banned ([Clean-room rules](Development.md#clean-room-rules)).
- **Drop-in**: reads DSN and writes SES like Freerouting, accepts Freerouting's command line, and integrates with KiCad directly through its IPC API.

## Pages

| Page | Covers |
| --- | --- |
| [Research](Research.md) | State-of-the-art routing algorithms, prior art, KiCad integration and the tech stack, with sources |
| [Architecture](Architecture.md) | The iDesign decomposition: [volatilities](Architecture/Volatilities.md), components, [use cases](Architecture/Use-cases.md), [contracts](Architecture/Contracts.md), the [routing pipeline](Architecture/Routing-pipeline.md), [parallelism and GPU](Architecture/Parallelism-and-GPU.md) |
| [Project plan](Project-plan.md) | Milestones, the dependency network of the tasks and the critical path |
| [Development](Development.md) | Repository layout, layer rules, craftsmanship, building, testing and benchmarking |
| [Profiles](Profiles.md) | Trading speed against quality: the built-in profile and `traxide.toml` |
| [Benchmarks](Benchmarks.md) | What traxide achieves on the test boards, milestone by milestone |
| [Glossary](Glossary.md) | The routing and design terms used here |
