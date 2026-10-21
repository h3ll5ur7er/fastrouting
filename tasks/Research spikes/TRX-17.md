---
id: TRX-17
title: "Spike: KiCad IPC API from a native exec plugin"
type: research
status: review
priority: medium
labels: [kicad, spike]
estimate: 1.5
---

Prove the in-editor path before building it ([[Research/KiCad-integration]]). Prototype: `spikes/kicad-ipc/` (outside the workspace).

- [x] A Rust `exec` plugin that reads layers, nets, net classes and pad polygons on KiCad 10 (10.0.7), with a hand-written client instead of `kicad-ipc-rs` (both tried)
- [x] Create a track and a via in one commit; check one undo removes both
- [x] Run KiCad 10 without a person: `pcbnew` in `kicad/kicad:10.0` under `xvfb-run`, first-run wizard suppressed (`run-headless.sh`, about 5 s)
- [x] Install through PCM's *Install from File* on Linux: executable bit kept, toolbar icons shown, the action commits through the API
- [ ] Install on Windows and macOS; check the named pipe, the toolbar icon and macOS quarantine (moved to [[TRX-33]], which builds those packages)
- [x] Note what KiCad 11 (master) changes: design rules over IPC, headless mode
- [x] Settle the licence question: messages are defined from field numbers, so no `.proto` text enters traxide; whether to ask the KiCad developers anyway is the maintainer's call

**Findings.** KiCad passes `KICAD_API_SOCKET` as an NNG address (`ipc://…`), not a path, starts an exec action from its folder with no arguments, and sets `KIPRJMOD` to the project. NNG's REQ framing is simple enough to write in about 100 lines of pure Rust, which avoids building C libnng with CMake for every release target. KiCad 10 serves net classes but not the board's minimum rules or custom rules, which come from the project files. With the API off, the plugin's buttons are not shown at all. Details and sources: [[Research/KiCad-integration#the-ipc-api]].

**Decisions for the maintainer.** Hand-written client (recommended) or `kicad-ipc-rs`; whether to ask the KiCad developers to confirm that clients of the API may be permissively licensed; how a single binary serves several actions without arguments (one entry point per action).

**Depends on:** [[TRX-1]]
