# traxide

*trace* + *oxide*: a fast, dependency-free PCB autorouter for KiCad and every tool that speaks Specctra DSN: a modern alternative to Freerouting, written in Rust, using every CPU core and, when present, the GPU.

> traxide is young: it routes a 4-layer keyboard with an RP2350 completely in seconds, and KiCad 10's DRC finds the result clean, while Freerouting takes minutes and leaves violations. It reads KiCad's board files and Specctra DSN, and has met few boards yet. See the [benchmarks](docs/Benchmarks.md).

## Try it

Every push builds traxide for Linux, Windows (x86-64 and ARM64) and macOS: open the latest run of the **Binaries** workflow under the repository's *Actions* tab and download the archive for your platform. Or build it:

```bash
cargo build --release
# A KiCad 9 or 10 board, with the rules of its project: writes board.routed.kicad_pcb
target/release/traxide route board.kicad_pcb
# Or, as with Freerouting: in KiCad, File → Export → Specctra DSN, then
target/release/traxide route board.dsn        # writes board.ses, on all cores
target/release/traxide route --help           # profiles, threads, output
target/release/traxide route board.dsn --time-limit 60   # stop negotiating after a minute
target/release/traxide check board.dsn board.ses   # complete? rules broken?
# The same verdict as a DRC report in KiCad's JSON format (route takes --drc too)
target/release/traxide check board.dsn board.ses --drc board.drc.json
# In place of Freerouting: the same arguments, -drc report.json included
target/release/traxide freerouting -de board.dsn -do board.ses
# In KiCad: File → Import → Specctra Session
```

### From inside KiCad

traxide's KiCad plugin adds a **Route board** button to the PCB editor's toolbar (KiCad 9 and 10). It routes every unrouted connection of the open board and adds the routes as one step: *Edit → Undo traxide: route* takes them all back. The rules come from KiCad's net classes and the project's files. On a board with copper zones traxide then refills them around the routes, a step of its own, so taking the routes back takes two undos.

1. **Enable KiCad's API first**: *Preferences → Preferences… → Plugins → Enable KiCad API*. It is off by default, and **without it KiCad shows no traxide button at all**.
2. Download the package for your platform (`traxide-kicad-<platform>.zip`) and install it with *Plugin and Content Manager → Install from File…*.
3. Restart the PCB editor: the button appears in its top toolbar. What traxide reports appears in KiCad's message panel (KiCad 10.0.1 and later).

The package carries the traxide binary itself; there is no Python. From a terminal, `traxide route --kicad` does what the button does.

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
