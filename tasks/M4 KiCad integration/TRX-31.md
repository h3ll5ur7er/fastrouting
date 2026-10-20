---
id: TRX-31
title: Talk to a running KiCad through the IPC API in Board Access
type: feature
status: todo
priority: high
labels: [board-access, kicad]
estimate: 5
layer: access
---

The `kicad-ipc` adapter, built on what [[TRX-17]] proved ([[Research/KiCad-integration#the-ipc-api]], prototype in `spikes/kicad-ipc/`).

- [ ] Client in Board Access: NNG REQ over a Unix socket, a Windows named pipe on Windows; `prost` messages defined from field numbers, no `.proto` text copied
- [ ] Socket from `KICAD_API_SOCKET` (an `ipc://` address) else the default `api.sock` in the temporary directory; token from `KICAD_API_TOKEN`; retry on `AS_BUSY` and `AS_NOT_READY` with a time limit
- [ ] Read the board: enabled layers and stackup, outline (`Edge.Cuts` shapes), footprints and pads with `GetPadShapeAsPolygon` per copper layer, nets, net classes (`GetNetClassForNets`), tracks, arcs and vias with their locked state, zones and rule areas as keepouts
- [ ] Rules: net classes over IPC; board minimums from `.kicad_pro` and custom rules from `.kicad_dru` in the project folder (`KIPRJMOD`, else the project path of `GetOpenDocuments`), reusing the `kicad` module; `GetBoardDesignRules` and `GetCustomDesignRules` on KiCad 11
- [ ] Hold the result to the board-file reader on the same boards: same design from IPC as from `.kicad_pcb`
- [ ] Store routes as one commit (`BeginCommit`, `CreateItems`, `EndCommit` with a message such as "traxide: route"), so one undo removes them; drop the commit when creating fails; then `RefillZones`
- [ ] Clear error when the API is unreachable, with the steps to enable it (*Preferences → Preferences… → Plugins → Enable KiCad API*, then restart the PCB editor)
- [ ] `cargo xtask e2e` drives the IPC path through KiCad 10 in Docker under `xvfb-run` (as `spikes/kicad-ipc/run-headless.sh` does) and judges the result with KiCad's DRC

**Depends on:** [[TRX-30]], [[TRX-17]]
