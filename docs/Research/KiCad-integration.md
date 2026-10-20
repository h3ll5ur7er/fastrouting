# KiCad integration

The three ways traxide meets KiCad, what each can and cannot do in KiCad 9, 10 and the coming 11, and which Board Access adapter implements it.

[TOC]

## Paths

| Path | How | Pros | Cons | Adapter |
| --- | --- | --- | --- | --- |
| Specctra DSN/SES | Export DSN from KiCad, route, import SES | Universal, what Freerouting does, how corpora compare | Lossy: custom rules not exported, quirky keepouts; KiCad 10 exports from the GUI (SWIG scripting goes away in KiCad 11) | `dsn` |
| Board files | Read `.kicad_pcb`, `.kicad_pro`, `.kicad_dru`; write tracks and vias back | Headless, CI, KiCad closed; complete rules | Must follow KiCad's file format versions | `kicad-file` |
| IPC API | Talk to the running KiCad | In-editor, undoable, live progress | Needs the GUI; off by default | `kicad-ipc` |

## The IPC API

- Protocol buffers over NNG: a Unix socket or a Windows named pipe from `KICAD_API_SOCKET`, with a token from `KICAD_API_TOKEN`. Since KiCad 9, mature in 10.
- **Disabled by default**: the user ticks *Preferences → Plugins → Enable API*. The plugin explains that on first run.
- KiCad 9 and 10 only serve a running GUI; headless use arrives with KiCad 11.
- KiCad 10.0's `board_commands.proto` has what routing needs: `GetNets`, `GetItemsByNet`, `GetNetClassForNets`, `GetBoardStackup`, `GetBoardEnabledLayers`, **`GetPadShapeAsPolygon`** (exact pad copper), `CheckPadstackPresenceOnLayers`, `RefillZones`, and creating items inside a commit, which KiCad undoes as one step.
- **Design rules are not served by 10.0**: `GetBoardDesignRules` and `GetCustomDesignRules` exist only on master (KiCad 11). On KiCad 10 the `kicad-ipc` adapter reads the board through IPC and the rules from `.kicad_pro` and `.kicad_dru` on disk. Board Access hides this mix — nothing above it knows.
- `kicad-ipc-rs` (MIT) covers all 59 commands of 10.0.1, including commit sessions.

## The plugin

KiCad's `plugin.json` (API v1) allows `runtime.type` **`"exec"`**: the plugin is a native executable with an entry point and arguments per action. traxide's KiCad plugin is therefore **the same binary, no Python**:

- There is no per-OS entry point, so there is one PCM package per platform; PCM's `metadata.json` lists platforms per version.
- KiCad 11 nightlies reportedly stop bundling Python, which makes Python plugins such as Freerouting's (which installs a JRE on demand) and OrthoRoute's (which builds a virtual environment) more fragile — an advantage for a native plugin.
- Konnect, a Rust KiCad plugin, already ships this way. An open kicad-python issue reports toolbar icons of PCM-installed IPC plugins not showing; test it ([[TRX-17]]).
- macOS: notarise the binary; whether PCM's download sets the quarantine attribute needs testing.

## Rules the DSN leaves out

KiCad's DSN export carries widths, clearances, net classes and vias, but not everything KiCad's DRC checks:

- **Copper to board edge.** KiCad applies its board setup, 0.5 mm by default. traxide keeps at least 0.5 mm on boards whose DSN names KiCad as `host_cad`, until it reads the real value from `.kicad_pro` ([[TRX-30]]).
- **Custom rules** (`.kicad_dru`): not exported at all; also [[TRX-30]].

## What is read

| Item | DSN | `.kicad_pcb` + project | IPC (KiCad 10) |
| --- | --- | --- | --- |
| Outline, stackup, layers | Yes | Yes | Yes |
| Pads with exact shapes | Approximated | Yes | Yes (`GetPadShapeAsPolygon`) |
| Nets, net classes, widths, clearances | Mostly | Yes (`.kicad_pro`) | Nets yes; rules from files |
| Custom rules (`.kicad_dru`) | No | Yes | From files |
| Keepouts, zones | Partly | Yes | Yes |
| Existing tracks and vias, locked state | Yes | Yes | Yes |

## Writing back

- DSN path: a SES file with wires and vias, imported by the user (or by Freerouting-style tools).
- File path: new `segment`, `arc` and `via` nodes in `.kicad_pcb`, every other byte kept; refuse to write while the board file changed since it was read.
- IPC path: one commit with all new items, so one undo removes the routing; then `RefillZones`.
