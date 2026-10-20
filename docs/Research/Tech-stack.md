# Tech stack

The requirements: Windows, macOS and Linux on x86-64 and ARM64; every core used, a GPU when present; one binary with **no runtime for the user to install** — no JVM, no Python, no GPU SDK.

[TOC]

## Language: Rust

| Criterion | Rust | Swift | Zig | C++ | Go |
| --- | --- | --- | --- | --- | --- |
| Six targets | Tier 1 on all | Strong on macOS, works elsewhere | Excellent cross-compilation | Good, toolchain pain | Excellent |
| One static binary | musl on Linux, `crt-static` on Windows, system libraries only on macOS | **Not fully static on Windows** (VC++ runtime); a static Linux SDK regression in 6.4 | Trivial | Possible, painful | Trivial |
| SIMD | `std::simd` still nightly; stable `fearless_simd`, `pulp`, `wide`; good autovectorisation on SoA loops | Built-in | `@Vector` | Intrinsics, Highway | Experimental |
| Threads | rayon (work stealing, deterministic reductions possible) | Actors | Manual | TBB, OpenMP | Goroutines, GC, races possible |
| Memory and data-race safety | Yes | Mostly, ARC overhead | No | No | GC |
| Geometry ecosystem | Best of the five (below) | Thin off Apple | Very thin | Strongest (Clipper2, CGAL, Boost) | Thin |
| WebAssembly, WebGPU | First class | Experimental | Good | Emscripten | Large binaries |
| GPU host | wgpu, pure Rust | Metal only in practice | By hand | By hand or Dawn | Weak |
| EDA contributors | Topola, AutoRoute, KiCadRoutingTools, kicad-ipc-rs are Rust | Few off Apple | Few, pre-1.0 | KiCad's developers | Some |

**Decision: Rust**, stable toolchain, edition 2024, a pinned minimum version. Swift fails the static Windows binary, Zig is pre-1.0 without a geometry ecosystem, Go has a GC and immature SIMD. C++ is the only real competitor and loses on safety, packaging and newcomers.

## Crates

| Need | Crate | Note |
| --- | --- | --- |
| Threads | `rayon` | The Work pool utility wraps it to keep reductions deterministic |
| SIMD | `fearless_simd` or `pulp` | Runtime dispatch per CPU, one binary for all x86-64 levels |
| Spatial index | own uniform grid; `rstar` for sparse items | Layout Access hides which |
| Predicates | `robust` | Shewchuk's exact predicates where integers don't suffice (arcs) |
| Polygon booleans, offsetting | `i_overlay`, `clipper2-rust` | Pure Rust; not the `clipper2` crate, which wraps C++ |
| Triangulation | `spade` | For a later topological backend |
| Arenas, scratch memory | `index_vec` or `slotmap`, `bumpalo` | Typed indices, SoA in hot loops; no ECS |
| GPU | `wgpu` with WGSL | See below |
| KiCad IPC | `kicad-ipc-rs` or own code generated with `prost` from KiCad's `.proto` files | |
| CLI, config, logs | `clap`, `serde` with `toml`, `tracing` | |
| Viewer | `egui` on `wgpu` | Optional client |
| Web | `wasm-bindgen`, `wasm-bindgen-rayon` | Threads need cross-origin isolation |

Graphs are our own compressed arrays, not `petgraph`, which suits small graphs only.

## GPU: wgpu, optional

wgpu (v30 in 2026) is the only GPU API that needs nothing but the system's drivers: Metal on macOS, DirectX 12 or Vulkan on Windows, Vulkan on Linux, WebGPU in browsers. Kernels are WGSL.

- **Limits to design for**: 16 KiB workgroup memory and 256 invocations are the portable minimum (read the adapter's real limits at run time); there is no global barrier inside a dispatch, so frontier searches take one dispatch per level or a persistent loop with a convergence flag.
- **No portable 64-bit atomics**: browsers have none, Metal only min/max on recent Apple GPUs. Kernels pack cost and tie-break into **32-bit** `atomicMin` (for example 20 bits of cost, 12 of tie-break) and use 64 bits only when the adapter has them.
- **Alternatives rejected for now**: CubeCL (one kernel for GPU and CPU, but heavy and churning; revisit), rust-gpu (alpha, nightly), CUDA in any form (NVIDIA only, needs an SDK), vulkano (no macOS without MoltenVK), Metal alone.

The GPU is an accelerator for data-parallel work over many nets at once — congestion and cost-map updates, global routing wavefronts, rasterising obstacles, batched DRC distances — never for single-net searches, where dispatch overhead loses to CPU A*. **Every GPU kernel has a CPU twin**, the reference for tests and the fallback. See [Parallelism and GPU](../Architecture/Parallelism-and-GPU.md).

## Formats

In the order they are built:

1. **Specctra DSN in, SES out**: parity with Freerouting, and how the public corpora are compared.
2. **`.kicad_pcb`, `.kicad_pro` and `.kicad_dru`** read and written directly: headless and CI routing with KiCad closed; footprints are embedded in the board, so no libraries are needed. Writing back adds `segment`, `arc` and `via` nodes and keeps every other byte. Custom rules need a small evaluator for their conditions (`A.NetClass == 'HV'`).
3. **The KiCad IPC API** of a running KiCad, for routing inside the editor with undo. See [KiCad integration](KiCad-integration.md).
4. Later: tscircuit's `SimpleRouteJson` (pairs well with the web demo), Eagle XML. Altium, Allegro and PADS boards go through KiCad's importers; IPC-2581 and ODB++ are manufacturing outputs, poor routing inputs.

## Distribution

- **cargo-dist** generates the release workflow, archives, installers and checksums; one extra job assembles the KiCad PCM packages.
- **Targets**:

| Target | Linking | GPU |
| --- | --- | --- |
| `x86_64`/`aarch64-unknown-linux-gnu` | glibc ≥ 2.28 (built with `cargo-zigbuild`), everything else static | Yes |
| `x86_64`/`aarch64-unknown-linux-musl` | Fully static | **No**: a static musl binary cannot load `libvulkan.so` |
| `x86_64`/`aarch64-pc-windows-msvc` | `+crt-static` | Yes |
| macOS universal (`lipo` of both) | System frameworks only; signed and notarised | Yes (Metal) |

- **Size**: `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `strip = true`, profile-guided optimisation for releases (~20 % faster in fastroute's experience). Expect a few MB CPU-only, several more with wgpu (estimate).

## Risks

1. **Quality decides adoption, not speed.** Freerouting improves quickly. Every release is gated on PCBench and PCBWorld completion and DRC-clean rates ([[FR-22]]).
2. **The GPU may not pay off** on typical boards. It is milestone M3 and benchmark-gated.
3. **KiCad IPC churn** between 10 and 11, the API being off by default, PCM icon issues ([[FR-17]]).
4. **Determinism under parallelism**: fixed batch order, seeded tie-breaks, tested ([[FR-27]]).
5. **Licence**: the repository is GPL-3.0, like Freerouting and KiCad, so Freerouting's ideas and code may be studied and reused; MIT crates such as Topola's are compatible. A permissive licence later would require avoiding Freerouting code.
