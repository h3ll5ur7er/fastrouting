# Prior art

What exists in October 2026, what each does well, and what fastrouting learns from it.

[TOC]

## Freerouting

GPL-3.0, Java 25, descended from Alfons Wirth's Specctra-like router. Versions 2.2 to 2.5 shipped in 2026; **v2.5.0 (5 October 2026) ships GraalVM native CLI binaries** for Linux x64, macOS and Windows x64 and a **KiCad 10 IPC mode**, and its next release aims to drop Java from the KiCad path.

> [!IMPORTANT]
> "No JVM" alone stops being a reason to switch. fastrouting has to win on **speed, use of all cores, routing quality and determinism**, measured on public corpora.

### How it routes

From the current sources (`app/freerouting/autoroute/`):

- **Free space as rooms and doors**: convex free-space polygons (`ExpansionRoom`), completed lazily, joined by `ExpansionDoor`s.
- **Search**: `MazeSearchEngine`, an A*-like search over rooms; the queue is a Java `TreeSet`; cost is per-layer trace cost, bend penalty and the rip-up cost of crossed obstacles; the estimate includes via costs.
- **Shove and rip-up**: crossing a trace first tries to shove it (bounded recursion), then queues it with a rip-up cost.
- **Batch loop**: plane nets, then signal nets, one connection at a time, over many passes with escalating rip-up costs (at least 8 passes; stops after 10 without progress); connections that leave clearance violations are rolled back.
- **Optimiser**: for each item, rip it and its connected traces up, reroute, keep the result if vias or weighted length improve. A thread pool exists but defaults to one thread, and every candidate deep-copies the board.

### Why it is slow

1. Routing passes are single-threaded and sequential per connection; multithreaded maze search is "future work" in the v2.5 notes.
2. Rooms are invalidated and rebuilt after every change, with costly polygon geometry.
3. Escalating rip-up costs are a slow, poorly converging stand-in for negotiated congestion, and there is no global plan.
4. Object-heavy geometry and a `TreeSet` queue strain the allocator and the garbage collector.
5. The optimiser reroutes everything, deep-copying the board per candidate.

Issue #289 records a board that took 4 h 46 min and 2.5 GB at about 11 % CPU. The project itself says jobs take "from a few minutes to several hours".

### Interfaces worth matching

- **CLI**: `-de board.dsn` in, `-do board.ses` out, `-mp` max passes, `-mt` optimiser threads, `-us greedy|global|hybrid`, `-inc` ignore net classes, `-drc` a KiCad-JSON DRC report, `--gui.enabled=false`. The [Freerouting-compatible CLI](../Architecture.md#clients) accepts these.
- **API server**: `--api_server.enabled=true` on `127.0.0.1:37864`. A later client could offer the same API.

## Other routers

| Project | Language, licence | What it is | Lessons |
| --- | --- | --- | --- |
| [sulkaharo/fastroute](https://github.com/sulkaharo/fastroute) | Rust, GPL-3.0 | A port of Freerouting: ~4–5× faster (510 s versus 2,102 s on 20 boards), 20–300 MB instead of 1–2 GB, byte-identical SES in an "exact mode" | A faithful port caps the gain at what better code buys; the algorithm is the bottleneck. **Its name is close to ours** |
| [pingpongshow/AutoRoute](https://github.com/pingpongshow/AutoRoute) | Rust, MIT | PathFinder-style negotiated congestion with rayon and egui, DSN/SES, `.kicad_dru` (without conditional rules) | Young (8 commits) but the same direction |
| [KiCadRoutingTools](https://github.com/atomic14/KiCadRoutingTools) | Rust core, Python, MIT | Octilinear grid A*, crossing-aware net ordering, rip-up with blocker analysis, Dubins differential pairs, length and time matching; reads and writes `.kicad_pcb` directly. 47 nets in 8.6 s | No push and shove, no global planning, no blind vias |
| [OrthoRoute](https://github.com/bbenchoff/OrthoRoute) | Python, CuPy/CUDA or Metal, MIT | GPU PathFinder on a Manhattan lattice (H/V layers, a via at every node) for huge backplanes, KiCad IPC plugin | Routes nets sequentially with a GPU SSSP per net. In PCBWorld it reached 30–51 % routability, far behind Freerouting. **A GPU doesn't buy quality**; its author calls it "not a general-purpose autorouter" |
| [tscircuit autorouter](https://github.com/tscircuit/tscircuit-autorouter) | TypeScript, MIT | A pipeline of solvers: capacity mesh (global grid) → pathing over nodes and ports → crossing-point optimisation (track assignment) → per-node detailed A* → untangling, stitching, repair; package-specific escape generators | **The closest public analogue to our [Routing pipeline](../Architecture/Routing-pipeline.md)** |
| [Topola](https://codeberg.org/topola/topola) | Rust, MIT | Topological rubber-band router: any-angle traces wrapping obstacles; DSN import, SES export, egui GUI; NLnet-funded | Work in progress (multilayer still incomplete in mid-2026); a candidate for collaboration on an any-angle backend later |
| KiCad's interactive router (PNS) | C++, GPL | Push and shove, differential pairs, length tuning inside pcbnew | Not usable as a library; KiCad 10 still has no built-in autorouter |
| DeepPCB, Quilter, Allegro X AI | Commercial, cloud | ML-assisted routing and layout | Vendor claims only; see [Algorithms](Algorithms.md#machine-learning) |

## Benchmark corpora

| Corpus | Size | Notes |
| --- | --- | --- |
| **PCBench** (in the Freerouting repository) | 1,157 boards | Freerouting v2.5.0 routes 75.4 % completely and 75.0 % with zero DRC violations; the corpus takes it 2,997 s |
| **PCBWorld-Bench** (LG AI Research, 2026) | 679 real KiCad boards plus generators | DRC-grounded metrics; Freerouting and KiCadRoutingTools as baselines |

Both are acceptance tests from the first milestone ([[FR-15]]), next to our own keyboard boards.

## Naming

"fastroute" (the Rust port above) and OpenROAD's FastRoute global router are close to "fastrouting". Not a blocker, but worth deciding before the first release ([[FR-35]]).
