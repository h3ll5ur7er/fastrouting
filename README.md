# traxide

*trace* + *oxide*: a fast, dependency-free PCB autorouter for KiCad and every tool that speaks Specctra DSN: a modern alternative to Freerouting, written in Rust, using every CPU core and, when present, the GPU.

> traxide is young: it routes a 4-layer keyboard with an RP2350 completely in seconds, and KiCad 10's DRC finds the result clean, while Freerouting takes minutes and leaves violations. It reads DSN and writes SES only so far, and has met few boards yet. See the [benchmarks](docs/Benchmarks.md).

## Try it

```bash
cargo build --release
# In KiCad: File → Export → Specctra DSN, then
target/release/traxide route board.dsn        # writes board.ses, on all cores
target/release/traxide route --help           # profiles, threads, output
# In place of Freerouting: the same arguments
target/release/traxide freerouting -de board.dsn -do board.ses
# In KiCad: File → Import → Specctra Session
```

A copy of the binary named `freerouting` (`freerouting.exe` on Windows) takes Freerouting's arguments too, for tools that run Freerouting.

- **Wiki**: [`docs/Home.md`](docs/Home.md). Covers [research](docs/Research.md), [architecture](docs/Architecture.md) (iDesign, decomposed by volatility) and the [project plan](docs/Project-plan.md).
- **Tasks**: [`tasks/`](tasks), one Markdown file per task, grouped by milestone.

Both are plain files managed with [Genbeya](https://github.com/genbeya/genbeya). Open this folder in Genbeya to get the wiki and the board with live references.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in traxide by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions. traxide is written clean-room: see [Clean-room rules](docs/Development.md#clean-room-rules).
