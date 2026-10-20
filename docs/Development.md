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
xtask/                       repository tasks: layer check, end-to-end test, later corpus and release
e2e/kicad/                   the KiCad half of the end-to-end test: board recipes, import, export
fixtures/boards/             small DSN boards shared by the tests of several crates
corpus/                      benchmark boards and baselines (Git LFS), with [[TRX-15]]
docs/                        this wiki
tasks/                       the task board
```

Crates are named `traxide-<component>` (`traxide-detailed`, `traxide-board-access`). The binary is `traxide`.

## Layer rules

The call rules of the [Architecture](Architecture.md#rules-between-components) are checked by `cargo xtask layers`, which reads `cargo metadata` and fails when a crate depends on one its layer may not use:

- clients → managers, contracts, utilities
- managers → engines, access, contracts, utilities
- engines → contracts, utilities. An engine uses Layout Access and Compute Access only through their **traits** in `traxide-contracts`; the manager hands it the implementation (dependency inversion), so an engine crate never depends on an access crate
- access → contracts, utilities, third-party crates for their resource (never another access crate)
- utilities → other utilities at most; contracts → geometry only

Crates appear when the task that needs them starts, not before (YAGNI): the layout above is the destination, not a scaffold.

CI runs the check on every pull request, next to `cargo fmt --check`, `cargo clippy -- -D warnings` and the tests.

## Building

```bash
cargo build --release                                  # the traxide binary in target/release
cargo run --release -p traxide-cli -- route board.dsn  # writes board.ses next to it
cargo test --all-features                              # unit and end-to-end tests
cargo xtask layers                                     # the layer rules
cargo xtask e2e                                        # the KiCad round trip, see below
```

Release builds use `lto = "fat"`, `codegen-units = 1` and `panic = "abort"`. Once the GPU backend exists ([[TRX-28]]) it will be a default feature, and `--no-default-features` will build a CPU-only binary.

`traxide route` exits with 0 when every connection is routed, 2 when some are not, 3 when the independent check finds a broken rule (a bug: please report it with the board) and 1 on errors.

## Craftsmanship

How code is written here, in order of importance:

1. **Test first.** Red, green, refactor. A behaviour is specified by a failing test before it is built, and a bug by a failing test before it is fixed. Tests are named after the behaviour they pin down (`routes_around_a_pad_of_another_net`), not after the function they call.
2. **Names instead of comments.** A well-chosen name for a file, type, function or variable replaces a paragraph. Extract a function or a variable when a name would explain what a comment otherwise would.
3. **Comments say why, never what.** The code says what it does. A comment earns its place by explaining a reason the code cannot show: a constraint, a trade-off, a quirk of KiCad, a reference to a paper. Doc comments on public items state the contract (units, invariants, errors), not the implementation.
4. **KISS and YAGNI.** The simplest thing that passes the tests and fits the architecture. No speculative generics, options or crates; extension points exist where [[Architecture/Volatilities]] says change is expected, and nowhere else.
5. **DRY for knowledge, WET for code until the third time.** Every rule, unit conversion and format quirk has one home. Similar-looking code is merged on the third occurrence, once the shared concept is clear. Tests may repeat themselves when that keeps each test readable on its own.
6. **SOLID, the Rust way.** One responsibility per module and type; traits only at component boundaries and volatilities (open/closed); small traits a client can implement fully (interface segregation); engines depend on access traits, not implementations (dependency inversion).
7. **Make illegal states unrepresentable.** Newtypes for ids and units (`Nm`, `NetId`), enums instead of flags, constructors that validate. A value that exists is valid.
8. **Errors are values.** Libraries return `Result` with `thiserror` types that say what went wrong and where (file, line, item); the client reports them with `anyhow`. `unwrap` and `expect` only where an invariant guarantees success, and `expect` says which.
9. **Deterministic always.** No iteration over hash maps in results, no time or address dependence; ties break on ids.

## Testing

Three levels, all run by `cargo test` except where noted:

- **Unit tests** next to the code. Engines are pure and take contracts, so most tests build a small `Design` in code.
- **End-to-end tests** in `tests/` of the CLI crate: run the `traxide` binary on DSN files and check the SES it writes — every connection routed, no clearance violation.
- **Real-world tests** through KiCad: `cargo xtask e2e` builds the board recipes of `e2e/kicad/boards.py` with KiCad's Python API, exports them as DSN, routes them with `traxide`, imports the sessions back into KiCad and requires KiCad's own DRC (`kicad-cli pcb drc`) to report no violations and no unconnected items. KiCad, not our code, is the judge. Results land in `target/e2e/<recipe>/`.
  - By default KiCad runs in the official Docker image `kicad/kicad:10.0`; `TRAXIDE_KICAD_IMAGE=kicad/kicad:9.0` tests KiCad 9. CI runs both.
  - `cargo xtask e2e --local` uses an installed KiCad instead: `TRAXIDE_KICAD_PYTHON` names a Python that can `import pcbnew` (on macOS the one inside `KiCad.app`), `TRAXIDE_KICAD_CLI` the `kicad-cli`.
  - Name recipes to run only those: `cargo xtask e2e crossing matrix-6x17`. The default set stays fast; bigger boards are benchmarks ([[Benchmarks]]).
  - KiCad 7 and 8 are not supported: they are not what people route with, and KiCad 7 cannot import sessions from a script.
- **Ground truth** from KiCad: `e2e/kicad/fixtures.py` exports boards together with KiCad's own placement of every pad; `crates/access/board/tests/kicad_ground_truth.rs` holds the DSN reader to it.
- **Test designs** in code: `traxide-contracts` with the `testing` feature offers `DesignBuilder`, so engine tests build boards in a few lines.
- **Property tests** (`proptest`) for geometry and for Layout Access: random shapes, random concurrent transactions.
- **Golden tests** for Board Access: parse, write, parse again, compare; and fixed DSN files with expected SES output.
- **Determinism tests**: route the same board with 1, 4 and all threads, CPU and GPU, and require identical routes.
- **Verification**: every routing test ends with a clearance check (the Verification Engine once it exists); a violation fails the test.

## Benchmarking

Until the benchmark exists ([[TRX-22]]), [[Benchmarks]] records measurements by hand. Then `cargo xtask bench` runs the Inspection Manager over `corpus/` and prints, per board, time, completion, vias, wire length and violations, next to the stored baselines (Freerouting's result and the last release's). CI runs it on the main branch and keeps the history, so a slowdown or a drop in completion shows up in the pull request that caused it.

## Clean-room rules

traxide is licensed `MIT OR Apache-2.0` ([LICENSE-MIT](../LICENSE-MIT), [LICENSE-APACHE](../LICENSE-APACHE)), so that anyone, including companies that ban copyleft, can use it. That only holds if no GPL code gets in:

- **Don't read the source of GPL routers while writing traxide's code**: Freerouting, KiCad's router (PNS), the GPL Rust port of Freerouting. Learn from papers, documentation, their observable behaviour and the summaries in [[Research/Prior-art]] — ideas are free, code is not.
- **Formats and interfaces are fair to implement from their specifications**: Specctra DSN/SES, KiCad's file formats, Freerouting's command line flags and API.
- **Dependencies must be permissive** (MIT, Apache-2.0, BSD, ISC, Zlib, Unicode, MPL-2.0 for unmodified files). `cargo deny check licenses` enforces this in CI. Code from MIT projects such as Topola, KiCadRoutingTools or AutoRoute may be reused with its copyright notice.
- **Corpus boards** are test data, not part of the binary; each keeps its own licence in `corpus/`.
- Contributions are accepted under both licences (inbound = outbound), stated in the contribution guide.

## Commits and branches

Name the task in commit messages and pull requests (`TRX-12: Route connections with multi-layer A*`). Tasks and wiki pages are committed with the work they describe.
