# Research

What we learned before designing fastrouting (October 2026), and the decisions it led to. The subpages hold the details and the sources.

| Page | Covers |
| --- | --- |
| [Algorithms](Research/Algorithms.md) | Maze, shape-based and topological routing, negotiated congestion, GPU global routers, ML, parallelism, optimisation |
| [Prior art](Research/Prior-art.md) | How Freerouting works and why it is slow, other routers (Rust ports, OrthoRoute, tscircuit, Topola), benchmark corpora |
| [Tech stack](Research/Tech-stack.md) | Language, crates, GPU, formats, distribution, risks |
| [KiCad integration](Research/KiCad-integration.md) | DSN/SES, board files and the IPC API; the native plugin |

## Findings

1. **Freerouting is slow by algorithm, not by language.** It routes one connection at a time, rebuilds its free-space rooms after every change, escalates rip-up costs instead of negotiating congestion, has no global plan, and optimises on deep copies of the board — on one thread. A faithful Rust port is only 4–5× faster.
2. **Chip design solved this a decade ago.** Global routing with negotiated congestion (PathFinder) on a coarse 3D grid, then detailed routing inside the resulting corridors, in parallel batches of non-overlapping regions. GPU routers (GAMER, FastGR, InstantGR) speed up the global stage on huge designs.
3. **A GPU doesn't buy quality**, and on a keyboard-sized board it hardly buys speed: per-net searches are tiny, so net- and region-level parallelism on CPU cores dominates. OrthoRoute, a GPU KiCad router, routes well under Freerouting's completion rate.
4. **ML doesn't replace search yet.** No published result beats engineered search with negotiation on DRC-clean completion; ML helps with priors, ordering and batching.
5. **The bar moved.** Freerouting 2.5 ships native binaries and an IPC mode; quality on public corpora (PCBench, PCBWorld) decides adoption.
6. **KiCad now takes native plugins.** IPC plugins can be executables (`runtime.type: exec`), so fastrouting needs no Python either.

## Decisions

| Decision | Because | Page |
| --- | --- | --- |
| Rust, one Cargo workspace, crates per component | Static binaries on six targets, safety, rayon, geometry crates, WASM | [Tech stack](Research/Tech-stack.md) |
| Integer nanometre coordinates | Exact, deterministic, like KiCad | [Contracts](Architecture/Contracts.md) |
| Global routing with negotiated congestion, then corridor-bounded detailed routing | State of the art; removes blind rip-up passes | [Routing pipeline](Architecture/Routing-pipeline.md) |
| CPU-first parallelism; GPU optional through wgpu, with a CPU twin per kernel | No SDKs; GPU pays off only for large boards | [Parallelism and GPU](Architecture/Parallelism-and-GPU.md) |
| Deterministic results on any number of threads | Reproducible boards and bug reports | [Parallelism and GPU](Architecture/Parallelism-and-GPU.md) |
| DSN/SES first, then KiCad files, then the IPC API | Freerouting parity and corpora first; headless next; in-editor last | [KiCad integration](Research/KiCad-integration.md) |
| An independent Verification Engine gates every result and every release | Quality decides adoption | [Use cases](Architecture/Use-cases.md) |
| No ML in the first milestones; hooks in Planning and Global Routing | No evidence it beats search yet | [Algorithms](Research/Algorithms.md#machine-learning) |
