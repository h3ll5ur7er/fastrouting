# Development

How the repository is laid out, how the layer rules are enforced, and how to build, test and benchmark. The choices behind the stack are in [Tech stack](Research/Tech-stack.md).

[TOC]

## Repository layout

One Cargo workspace; one crate per component, in folders named after the iDesign layers:

```text
crates/
  contracts/                 data and service contracts
  utilities/
    geometry/                integer geometry, predicates, offsetting
    diagnostics/             tracing setup, counters, timings
    events/                  progress events, cancellation, job queue
    pool/                    work pool, deterministic reductions
  access/
    board/                   DSN/SES, .kicad_pcb, KiCad IPC API adapters
    layout/                  the routing state: spatial index, transactions
    compute/                 CPU (rayon, SIMD) and GPU (wgpu) backends
    profile/                 profiles, traxide.toml, overrides
    run/                     reports, checkpoints, corpus, baselines
  engines/
    rules/  planning/  global/  detailed/  optimization/  verification/
  managers/
    routing/  inspection/
  clients/
    cli/                     the traxide binary, including the Freerouting-compatible mode
    kicad/                   the KiCad PCM package around the binary
    viewer/                  optional live viewer
    wasm/                    the web demo
xtask/                       repository tasks: layer check, corpus, release
corpus/                      benchmark boards and baselines (Git LFS)
docs/                        this wiki
tasks/                       the task board
```

Crates are named `traxide-<component>` (`traxide-detailed`, `traxide-board-access`). The binary is `traxide`.

## Layer rules

The call rules of the [Architecture](Architecture.md#rules-between-components) are checked by `cargo xtask layers`, which reads `cargo metadata` and fails when a crate depends on one its layer may not use:

- clients → managers, contracts, utilities
- managers → engines, access, contracts, utilities
- engines → `traxide-layout-access`, `traxide-compute-access`, contracts, utilities (never another engine)
- access → contracts, utilities, third-party crates for their resource (never another access crate)
- utilities → contracts at most; contracts → geometry only

CI runs the check on every pull request, next to `cargo fmt --check`, `cargo clippy -- -D warnings` and the tests.

## Building

```bash
cargo build --release            # the traxide binary in target/release
cargo run -p traxide-cli -- route board.dsn -o board.ses
```

Release builds use `lto = "fat"`, `codegen-units = 1` and `panic = "abort"`. The GPU backend is a default feature (`gpu`); `--no-default-features` builds a CPU-only binary.

## Testing

- **Unit tests** next to the code. Engines are pure and take contracts, so most tests build a small `Design` in code.
- **Property tests** (`proptest`) for geometry and for Layout Access: random shapes, random concurrent transactions.
- **Golden tests** for Board Access: parse, write, parse again, compare; and fixed DSN files with expected SES output.
- **Determinism tests**: route the same board with 1, 4 and all threads, CPU and GPU, and require identical routes.
- **Verification**: every routing test ends with the Verification Engine; a violation fails the test.

## Benchmarking

`cargo xtask bench` runs the Inspection Manager over `corpus/` and prints, per board, time, completion, vias, wire length and violations, next to the stored baselines (Freerouting's result and the last release's). CI runs it on the main branch and keeps the history, so a slowdown or a drop in completion shows up in the pull request that caused it.

## Clean-room rules

traxide is licensed `MIT OR Apache-2.0` ([LICENSE-MIT](../LICENSE-MIT), [LICENSE-APACHE](../LICENSE-APACHE)), so that anyone, including companies that ban copyleft, can use it. That only holds if no GPL code gets in:

- **Don't read the source of GPL routers while writing traxide's code**: Freerouting, KiCad's router (PNS), the GPL Rust port of Freerouting. Learn from papers, documentation, their observable behaviour and the summaries in [[Research/Prior-art]] — ideas are free, code is not.
- **Formats and interfaces are fair to implement from their specifications**: Specctra DSN/SES, KiCad's file formats, Freerouting's command line flags and API.
- **Dependencies must be permissive** (MIT, Apache-2.0, BSD, ISC, Zlib, Unicode, MPL-2.0 for unmodified files). `cargo deny check licenses` enforces this in CI. Code from MIT projects such as Topola, KiCadRoutingTools or AutoRoute may be reused with its copyright notice.
- **Corpus boards** are test data, not part of the binary; each keeps its own licence in `corpus/`.
- Contributions are accepted under both licences (inbound = outbound), stated in the contribution guide.

## Commits and branches

Name the task in commit messages and pull requests (`TRX-12: Route connections with multi-layer A*`). Tasks and wiki pages are committed with the work they describe.
