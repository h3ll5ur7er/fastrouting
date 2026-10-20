# Algorithms

A survey of PCB and VLSI routing algorithms as of October 2026: what exists, what works, and what fastrouting takes from each. The pipeline that results is in [Routing pipeline](../Architecture/Routing-pipeline.md).

[TOC]

## Classical baselines

| Family | Idea | Strength | Weakness |
| --- | --- | --- | --- |
| **Maze routing** (Lee 1961, A*, Hadlock, Soukup) | Wavefront search on a grid; on a 3D grid (x, y, layer) vias are z-edges | Complete, optimal on its graph; still the inner loop of nearly every router, VLSI detailed routers such as TritonRoute and Dr. CU included | O(area) time and memory on a fine uniform grid |
| **Line probe** (Hightower 1969, Mikami–Tabuchi 1968) | Shoot escape lines instead of flooding cells | Little memory, fast on sparse boards | Incomplete, poor in congestion; mostly historical |
| **Gridless, shape-based** (corner stitching, Contour, Specctra 1989, BonnRoute) | Search free space as shapes or tiles, not cells | Arbitrary widths and clearances, exact geometry | Free-space structures are costly to maintain under change |
| **Rip-up and reroute** (Nair 1987 …) | Route in order; rip up blockers, reroute with rising costs | Simple | Sensitive to net order, oscillates; negotiated congestion is the principled version |

BonnRoute, the most refined modern detailed router, uses a shape-based structure for pin access and a track structure for long connections, with goal-oriented Dijkstra. Path search is 80–90 % of its run time, and a fast grid structure alone gave more than 5×: **the data structure behind the search matters as much as the search**.

## Negotiated congestion

**PathFinder** (McMurchie and Ebeling, 1995) prices every routing resource *n* as `(b_n + h_n) · p_n`:

- `b_n`, the base cost (length, via, wrong-way direction);
- `h_n`, the history cost, which grows in every iteration in which *n* is overused;
- `p_n`, the present-sharing penalty, which grows with the current occupancy and with the iteration.

Every net (in modern variants, every net touching overuse) is ripped up and rerouted each iteration until nothing is overused. The result is independent of net order in the limit. PathFinder is the basis of VPR and RWRoute for FPGAs and of essentially every VLSI global router.

### VLSI global routers

| Router | Contribution |
| --- | --- |
| FastRoute 2.0–4.1 (Iowa State, now OpenROAD `grt`) | Monotonic and multi-source maze routing, virtual capacity, via-aware Steiner trees, 3-bend routing with layer assignment (−13.8 % vias) |
| NCTU-GR 2.0 | Multithreaded, collision-aware, bounded-length maze routing |
| SPRoute (Yale) | Net-parallel negotiation that throttles parallelism on livelock: 11.5× on 28 cores for +0.6 % wire length |
| CUGR (DAC 2020) | 3D pattern routing with layer assignment, multi-level 3D maze routing |
| CUGR 2.0 / EDGE (DAC 2023) | DAG-based pattern routing that adds detours only when needed; single-threaded, faster than prior multithreaded routers |

### GPU global routers (2021–2026)

| Router | Contribution |
| --- | --- |
| GAMER (ICCAD 2021, TCAD 2023) | Shortest paths as alternating horizontal and vertical sweeps, each a parallel prefix-min: 16–20× on CUGR's maze stage, 2.7× end to end |
| FastGR (DATE 2022, TCAD 2023) | GPU L-shape pattern routing (9.3×) plus a CPU/GPU task-graph scheduler |
| InstantGR (ICCAD 2024, TCAD 2026, BSD-3) | Fully GPU 3D global router; ISPD'24 contest winner (special track) on designs of up to 50 M cells. Net batching is up to 17 % of its run time |
| GANGR (2025) | Learns better batching of non-overlapping nets |
| DGR (NVIDIA, Apache-2.0) | Relaxes the choice among candidate patterns into a continuous optimisation on the GPU, then hands the result to CUGR 2 |

**The state-of-the-art recipe**: Steiner decomposition (FLUTE) → massively parallel pattern routing (L, Z, 3-bend shapes with a layer DP) → negotiated rip-up and reroute of what still overflows, with maze search, in batches of non-overlapping bounding boxes → optionally a differentiable or Lagrangian pass.

## Topological, escape, exact and constrained routing

- **Topological (rubber-band) routing.** SURF (UCSC, 1991) decides topology first — which side of each obstacle a wire passes, on a constrained Delaunay triangulation — and geometry later. TopoR does it commercially, any-angle. Open attempts: [Topola](https://codeberg.org/topola/topola) (Rust, MIT, work in progress) and an abandoned Ruby/CGAL router. Excellent length and smooth shapes, but realising robust, DRC-clean geometry is hard and no open-source one is robust yet. *A candidate for a later detailed routing backend, not for the first.*
- **Any-angle search.** Theta* and Lazy Theta* give any-angle paths on grids. For PCBs, octilinear (0°, 45°, 90°) is the practical sweet spot; KiCad's interactive router is built on octagons.
- **BGA/QFN escape.** Classically min-cost multi-commodity flow. For an RP2350 (QFN-60 or QFN-80, 0.4 mm pitch) escape is a pattern problem — pad, short stub, via or perimeter — and needs no flow solver.
- **ILP/SAT.** OptRouter (ILP) and SAT routers are optimal and can *prove* that a small region is unroutable, but only scale to small windows: a **local repair oracle**, not the main engine.
- **Differential pairs and length matching.** Route a pair as one coupled entity (centre line plus gap), with pose-based A* (position and heading, Dubins heuristic) as in KiCadRoutingTools; tune length afterwards in reserved meander regions (obstacle-aware length matching, 2024). For USB 2.0 on a keyboard, coupling matters far more than skew.

## Machine learning

| Approach | State in 2026 |
| --- | --- |
| DeepPCB (InstaDeep) | Cloud RL router. Vendor benchmarks only; an independent EEVblog test (2023) saw it fail a board after 2 h that Freerouting finished in 10 min |
| Quilter | Physics-driven RL generating and evaluating many candidate layouts; vendor claims only (an i.MX8M board 98 % automated in 27 h plus 38 h of cleanup) |
| Cadence Allegro X AI | Generative placement and critical-net routing; internals undisclosed |
| Academic (PRNet, HubRouter, DSBRouter, FanoutNet, CircuitNet predictors) | Generative global routing still needs a classical reroute step; congestion predictors reach Pearson ≈ 0.64 |
| PCBWorld (LG AI Research, 2026) | RL environment on KiCad's router with KiCad DRC rewards and 679 real boards: its policy *approaches* rule-based routers, it doesn't beat them |

**Verdict**: no published evidence shows learned routers beating well-engineered search with negotiation on DRC-clean completion. ML is useful *around* the search — predicted congestion as a prior for history costs, net ordering and batching policies, candidate ranking — never instead of the exact, DRC-checked inner loop. fastrouting leaves room for that in the Planning and Global Routing engines and builds none of it first.

## Parallelism

1. **Batches of nets with non-overlapping bounding boxes**, routed in parallel within each negotiation iteration (FastGR, InstantGR, Dr. CU). Deterministic when batches come from a deterministic colouring and merge in a fixed order; a DATE 2020 FPGA scheduler is serial-equivalent at ~19×.
2. **Speculative routing with rollback**: route against a snapshot, detect conflicts at commit, retry. High throughput; deterministic only with a fixed commit order.
3. **Region decomposition**: global routing fixes corridors and crossing points on region borders, then each region is routed independently and stitched (tscircuit, TritonRoute panels).
4. **Parallel search inside one net**: GPU SSSP (near-far, delta-stepping, GAMER sweeps), SIMD wavefronts over bit-packed rows. Pays off only on large grids. On a keyboard board each net is tiny, so **net and region parallelism dominate**.

See [Parallelism and GPU](../Architecture/Parallelism-and-GPU.md) for what fastrouting does with this.

## Post-routing optimisation

- **Pull-tight**: shorten a trace within its homotopy class against inflated obstacles (a funnel algorithm in the channel it already uses), then snap to octilinear.
- **Via minimisation**: layer reassignment of segments as 2-colouring or max-cut on a conflict graph (ILP for small instances), plus removing via pairs where one layer suffices.
- **Mitering**: 90° corners to 45° chamfers where clearance allows.
- **Finishing**: teardrops, stub removal, stitching vias next to layer changes of the USB pair.
- Freerouting's optimiser — rip up each item, reroute it, keep it if better — is sound in concept; it is slow because it is sequential and deep-copies the board per candidate. fastrouting does region-bounded moves in parallel on copy-on-write regions.

## References

- PathFinder: [course copy](https://www.esa.informatik.tu-darmstadt.de/archive/twiki/pub/Lectures/AlgorithmenImChipEntwurfDe/router-pathfinder.pdf)
- Contour gridless router: [WRL 95/3](https://ftp.mirrorservice.org/sites/www.bitsavers.org/pdf/dec/tech_reports/WRL-95-3.pdf); BonnRoute: [arXiv 2111.06169](https://arxiv.org/pdf/2111.06169)
- FastRoute: [OpenROAD grt](https://openroad.readthedocs.io/en/latest/main/src/grt/src/fastroute/README.html); SPRoute: [paper](https://csl.yale.edu/~rajit/ps/sproute.pdf); CUGR 2: [cu-gr-2](https://github.com/cuhk-eda/cu-gr-2)
- GAMER: [IEEE](https://ieeexplore.ieee.org/document/9643563/); FastGR: [TCAD 2023](https://www.cse.cuhk.edu.hk/~byu/papers/J86-TCAD2023-FastGR.pdf); InstantGR: [code](https://github.com/cuhk-eda/InstantGR); GANGR: [arXiv 2511.17665](https://arxiv.org/abs/2511.17665); DGR: [code](https://github.com/NVlabs/Differentiable-Global-Router); ISPD'24 contest: [NVIDIA](https://research.nvidia.com/publication/2024-03_gpuml-enhanced-large-scale-global-routing-contest)
- SURF: [UCSC-CRL-93-04](https://tr.soe.ucsc.edu/sites/default/files/technical-reports/UCSC-CRL-93-04.pdf); TopoR: [Wikipedia](https://en.wikipedia.org/wiki/TopoR); Topola: [Codeberg](https://codeberg.org/topola/topola)
- OptRouter: [DAC 2015](https://vlsicad.ucsd.edu/Publications/Conferences/328/c328.pdf); length matching: [arXiv 2407.19195](https://arxiv.org/abs/2407.19195)
- Parallel routing: [DATE 2020](https://www.date-conference.com/proceedings-archive/2020/pdf/0103.pdf), [ADDS, PPoPP 2021](https://www.cs.utexas.edu/~lin/papers/ppopp21.pdf)
- ML: [PCBWorld](https://github.com/LGAI-Research/PCBWorld), [HubRouter](https://proceedings.neurips.cc/paper_files/paper/2023/hash/f7f98663c516fceb582354ee2d9d274d-Abstract-Conference.html), [DSBRouter](https://proceedings.mlr.press/v267/shi25k.html), [EEVblog DeepPCB test](https://eevblog.com/forum/blog/eevblog-1535-deeppcb-ai-autorouting-tested!/msg4787999)
- KiCad's interactive router: [FOSDEM 2015](https://archive.fosdem.org/2015/schedule/event/pcb_routing/)

> [!WARNING]
> Not verified against primary texts: DGR's venue and authors, HeLEM-GR's method, PCBWorld's exact result tables, and every DeepPCB, Quilter and Cadence figure (vendor claims).
