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

What [[TRX-17]] verified against KiCad 10.0.7 (the `kicad/kicad:10.0` image) with the prototype in `spikes/kicad-ipc/`, and what it only read in the documentation. Sources: KiCad's developer documentation for add-on developers (the `kicad-dev-docs` repository), the protocol buffer definitions of KiCad 10.0.7 and of master read as an interface, the `plugin.json` schema `api/schemas/api.v1.schema.json`, `kicad-python`'s licence and changelog, `kicad-ipc-rs` 0.5.1 (MIT) and KiCad's observable behaviour. No KiCad source code was read.

### Transport

- **Protocol buffers over NNG's REQ/REP**, KiCad being the REP side. On Linux and macOS a Unix socket, on Windows a named pipe, created in the user's temporary directory as `kicad/api.sock`; a second KiCad appends its process id.
- A plugin launched by KiCad gets **`KICAD_API_SOCKET` as an NNG address, `ipc:///tmp/kicad/api.sock`, not a path**, and `KICAD_API_TOKEN`, a per-instance id to put in every request's header (a mismatch is refused, which tells a long-running client that KiCad restarted). A client started by hand has neither and falls back to the default path.
- **Framing** (the scalability protocols' IPC mapping, verified): both sides first send an 8-byte header `00 'S' 'P' 00`, protocol (`0x0030` REQ, `0x0031` REP), `00 00`; then each message is `0x01`, a big-endian `u64` length and the payload. REQ prefixes its payload with a 4-byte request id whose top bit is set, and REP echoes it.
- **Envelope**: `kiapi.common.ApiRequest { header { kicad_token, client_name }, message: Any }` and `ApiResponse { header, status { status, error_message }, message: Any }`. `Any.type_url` is `type.googleapis.com/` plus the full message name, such as `kiapi.board.commands.GetNets`.
- KiCad handles requests on its GUI thread: it answers **`AS_BUSY` while a modal dialog is open** (and `AS_NOT_READY` just after start), so a client retries. One connection per client; KiCad serialises requests anyway.
- **Speed**: the whole spike (about 12 requests, 247 pads, 370 tracks read twice, one commit, one undo) takes 0.6 s; reading items is not a bottleneck next to routing.

### What KiCad 10.0 serves for routing

| Need | Command (10.0) | Verified |
| --- | --- | --- |
| Board, project folder | `GetOpenDocuments` (board file name, project name and path); `KIPRJMOD` in a plugin's environment | Yes |
| Layers, stackup | `GetBoardEnabledLayers` (copper layer count), `GetBoardStackup` | Layer count |
| Nets | `GetNets` (by name; net codes are deprecated) | Yes (112) |
| Net classes with width, clearance, via stack, differential pairs | `GetNetClassForNets` | Yes |
| Pads | `GetItems` with `KOT_PCB_PAD`: id, number, net, position, padstack | Yes (247) |
| Exact pad copper | `GetPadShapeAsPolygon` per layer | Yes (straight segments) |
| Tracks, arcs, vias with locked state | `GetItems` with `KOT_PCB_TRACE`, `KOT_PCB_ARC`, `KOT_PCB_VIA` | Tracks, vias |
| Zones and keepouts | `GetItems` with `KOT_PCB_ZONE`; rule areas are `ZT_RULE_AREA` | No |
| Outline | `GetItems` with `KOT_PCB_SHAPE` on `Edge.Cuts` | No |
| Write routes | `BeginCommit`, `CreateItems`, `EndCommit` | Yes |
| Refill zones | `RefillZones` | No |

- **Design rules are not served by 10.0**: its `BoardDesignRules` message is empty and has no command; `GetBoardDesignRules` and `GetCustomDesignRules` exist on master (KiCad 11). Net-class values do come over IPC, so on KiCad 10 the `kicad-ipc` adapter reads from the project files (`KIPRJMOD`) only what net classes do not carry: the board minimums of `.kicad_pro` (`min_clearance`, copper to edge, hole clearance) and the custom rules of `.kicad_dru`. Board Access hides this mix; nothing above it knows.
- **Writing**: items created between `BeginCommit` and `EndCommit` (`CMA_COMMIT`) appear together and form **one undo step named by the commit's message** (*Edit → Undo traxide: route*). Verified: a track and a through via in one commit, removed by one undo. `CMA_DROP` discards a commit. A via needs its padstack (drill, copper size on each layer), a track its width, layer and net by name.
- `RunAction` runs an editor action by name; KiCad calls it a prototyping aid, not an API. The spike uses `common.Interactive.undo` only to play the user's Ctrl+Z.

### Client: hand-written, not `kicad-ipc-rs`

| | Hand-written (`spikes/kicad-ipc`) | `kicad-ipc-rs` 0.5.1 |
| --- | --- | --- |
| Licence of what enters the binary | prost, prost-types: Apache-2.0 | MIT and Apache-2.0 throughout (kicad-ipc-rs, nng, nng-sys with the bundled C libnng, tokio, prost) |
| Build | Pure Rust | Builds libnng (C) with CMake through `nng-sys` 1.4.0-rc.0, a release candidate; a C toolchain per target |
| Size | About 100 lines of transport plus the messages used; spike binary 0.8 MB | 21 crates with tokio; 0.7 MB for a bare connect |
| Coverage | What traxide needs, added per message | All of 10.0.1's commands, typed wrappers |
| KiCad's text in it | None: messages defined from field numbers and types | The generated code carries the `.proto` files' comments (GPL-3.0-or-later headers) |
| Maintenance | Ours; the API is stable by protobuf rules (fields are added, never changed) | One maintainer, releases since February 2026, pinned to 10.0.1 |

Both worked against KiCad 10.0.7. The recommendation is the hand-written client: no C build in the cross-compiled release binaries ([[TRX-35]]), no async runtime, and the licence question below does not arise.

**Licence**: KiCad's `.proto` files carry GPL-3.0-or-later headers. The KiCad developers themselves publish `kicad-python`, generated from those files, under MIT (relicensed in its 0.2.0, February 2025), which suggests they do not treat the API definitions as making clients GPL; no explicit statement was found in the documentation. Defining our own messages from the field numbers keeps traxide clear of the question either way: field numbers and types are the interface, not KiCad's text.

## The plugin

KiCad's `plugin.json` (schema `https://go.kicad.org/api/schemas/v1`) allows `runtime.type` **`"exec"`**: the plugin is a native executable, so traxide's KiCad plugin is **the same binary, no Python**. Verified on Linux with KiCad 10.0.7:

```json
{
  "$schema": "https://go.kicad.org/api/schemas/v1",
  "identifier": "com.github.h3ll5ur7er.traxide",
  "name": "traxide",
  "description": "Fast autorouter",
  "runtime": { "type": "exec" },
  "actions": [
    {
      "identifier": "route",
      "name": "Route board",
      "description": "Route every unrouted connection",
      "show-button": true,
      "scopes": ["pcb"],
      "entrypoint": "traxide",
      "icons-light": ["icon.png"],
      "icons-dark": ["icon-dark.png"]
    }
  ]
}
```

- Required: `identifier`, `name`, `description`, `runtime`, `actions`; each action `identifier`, `name`, `description`, `entrypoint`. **There are no arguments and no per-platform entry point**: each action needs its own entry point, so several actions of one binary need a small launcher per action or one binary that tells actions apart by name (a hard link or copy per action, or `argv[0]`). One PCM package per platform carries the right binary; PCM's `metadata.json` lists `platforms` per version.
- KiCad starts the entry point **from the plugin's folder as working directory, with no arguments**, and with `KICAD_API_SOCKET`, `KICAD_API_TOKEN`, `KIPRJMOD` (the project folder) and KiCad's `KICAD10_*` paths in the environment. Since 10.0.1 a plugin's stdout and stderr appear in the editor's message panel.
- KiCad 9 and 10 run IPC plugins only in the PCB editor.
- **Where plugins live**: KiCad loads `plugin.json` from subfolders of `${KICAD_DOCUMENTS_HOME}/10.0/plugins` (`~/.local/share/kicad`, `~/Documents/KiCad`, `C:\Users\<user>\Documents\KiCad`) and of PCM's `${KICAD_DOCUMENTS_HOME}/10.0/3rdparty/plugins`. PCM's *Install from File* unpacks the zip's `plugins/` into `3rdparty/plugins/<identifier with dots as underscores>` and `resources/` into `3rdparty/resources/<…>`; **the executable bit survives on Linux**, and the toolbar shows the action's icon, so the reported missing icons of PCM-installed IPC plugins did not reproduce there. Windows and macOS (including the quarantine attribute and notarisation) are untested.
- **What the user must do**: tick *Preferences → Preferences… → Plugins → Enable KiCad API* (stored as `api.enable_server` in `kicad_common.json`; off by default) and restart the PCB editor. Verified: with the API off there is no socket **and the plugin's buttons are not shown at all**, so the plugin cannot explain it itself; the PCM description, the README and a standalone `traxide` must. Ticking it starts the API at once, but the buttons appear only after the PCB editor restarts.
- KiCad 11 nightlies reportedly stop bundling Python, which makes Python plugins such as Freerouting's (which installs a JRE on demand) and OrthoRoute's (which builds a virtual environment) more fragile, an advantage for a native plugin. Konnect, a Rust KiCad plugin, already ships this way.

## Testing without a person

KiCad 9 and 10 serve the API only from the GUI, but the GUI runs without a screen: `spikes/kicad-ipc/run-headless.sh BOARD.kicad_pcb` starts `pcbnew` from the `kicad/kicad:10.0` image on an `xvfb-run` display (the image has no X server; the host's is shared through `/tmp/.X11-unix`, with `-ac` because the container's host name does not match the cookie), shares `/tmp/kicad` with the host for the socket, and runs the spike against it: from nothing to a verified commit and undo in about 5 seconds.

- KiCad greets a new configuration with a **first-run wizard, a modal dialog that keeps the API answering `AS_BUSY`**. It stays away when all three global library tables exist (`fp-lib-table` and `sym-lib-table` ship in the image, `design-block-lib-table` does not) and `kicad_common.json` marks its questions as answered: `"do_not_show_again": { "data_collection_prompt": true, "update_check_prompt": true }`, next to `"api": { "enable_server": true }`.
- A board left open by a killed KiCad keeps its `~*.lck` files, and the next start asks whether to open it anyway, again a modal dialog; delete them first.
- Linux only: Docker Desktop on macOS and Windows does not pass a Unix socket through a bind mount. GitHub's Linux runners have `xvfb-run`, so `cargo xtask e2e` can drive the IPC path the same way.

## KiCad 11

From master's protocol definitions and the documentation: design rules over IPC (`GetBoardDesignRules`, `GetCustomDesignRules`, new `board_rules.proto`), a headless API server in `kicad-cli`, jobs for plotting and export, and IPC plugins in the schematic editor. The message layout stays compatible (fields are added, not changed), so a client built for 10.0 keeps working; on 11 the adapter can ask for the rules instead of reading files.

## Rules the DSN leaves out

KiCad's DSN export carries widths, clearances, net classes and vias, but not everything KiCad's DRC checks:

- **Copper to board edge.** KiCad applies its board setup, 0.5 mm by default. From a DSN, traxide keeps at least 0.5 mm on boards whose DSN names KiCad as `host_cad`; from a board file it reads `min_copper_edge_clearance` from `.kicad_pro`.
- **Custom rules** (`.kicad_dru`): not exported at all, and not read from board files yet ([[TRX-32]]).

## Reading board files

The `kicad` module of Board Access reads `.kicad_pcb` as KiCad 9 (nets by number) and 10 (nets by name) write it, with the rules of the `.kicad_pro` beside it, or KiCad's defaults without one ([[TRX-30]]). Tests check it against KiCad's own placement of every pad and against KiCad's DSN export of the same boards: pads, nets, layers, rules, outline and keepouts agree.

- **Coordinates** are KiCad's: millimetres to the nanometre, y pointing down; a pad's `at` angle includes its footprint's.
- **Rules**: each net class's width, clearance and via, raised to the board's minimums; a class leaves out what it takes from `Default`; nets join classes by `netclass_assignments`, then by the wildcards of `netclass_patterns`.
- **Holes without copper** keep everything out by the board's hole clearance (`min_hole_clearance`) all around, as KiCad's DSN export writes them; without it KiCad's DRC finds hole-clearance violations.
- **Shapes** cover KiCad's and are never smaller: rounded rectangles as octagons that touch the corners, trapezoids and chamfered rectangles as their bounding rectangles, custom pads as their anchor and primitives. Arcs become segments within 2 µm of the arc. Per-layer padstacks use the front shape.
- **Outline**: the drawings on Edge.Cuts, joined end to end; loops inside the outline are cutouts, kept out like keepouts. Copper zones are left out, as KiCad refills them; rule areas that forbid tracks or vias are keepouts.
- **Copper of no net** keeps other copper away, as keepouts.

Routes are written back as `segment` and `via` nodes before the board's closing parenthesis; every other byte stays. Board Access reads the board again first and refuses to write if it no longer gives the same design. By default `traxide route BOARD.kicad_pcb` writes `BOARD.routed.kicad_pcb` and copies the project beside it, so that KiCad opens the routed board with its rules.

## What is read

| Item | DSN | `.kicad_pcb` + project | IPC (KiCad 10) |
| --- | --- | --- | --- |
| Outline, stackup, layers | Yes | Yes | Yes |
| Pads with exact shapes | Approximated | Yes | Yes (`GetPadShapeAsPolygon`) |
| Nets, net classes, widths, clearances | Mostly | Yes (`.kicad_pro`) | Nets and net classes yes; board minimums from `.kicad_pro` |
| Custom rules (`.kicad_dru`) | No | Yes | From files |
| Keepouts, zones | Partly | Yes | Yes |
| Existing tracks and vias, locked state | Yes | Yes | Yes |

## Writing back

- DSN path: a SES file with wires and vias, imported by the user (or by Freerouting-style tools).
- File path: new `segment`, `arc` and `via` nodes in `.kicad_pcb`, every other byte kept; refuse to write while the board file changed since it was read.
- IPC path: one commit with all new items, so one undo removes the routing; then `RefillZones`.
