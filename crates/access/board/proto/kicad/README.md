# KiCad's API definitions

The protocol buffer definitions of KiCad's IPC API that traxide talks to a running KiCad with, copied unchanged from KiCad's repository: `api/proto/` of the tag [`10.0.7`](https://gitlab.com/kicad/code/kicad/-/tree/10.0.7/api/proto), the KiCad that `cargo xtask e2e` runs. Only the files traxide needs are here: the envelope, the editor's and the board's commands, and what those import.

The build script of `traxide-board-access` compiles them into Rust with `protox` and `prost-build`, without their comments.

**Licence**: these files are part of KiCad and keep its GPL-3.0-or-later headers. They are the one exception to the clean-room rules, decided by the maintainer ([[Development#clean-room-rules]], [[TRX-17]]): many permissively licensed clients, such as KiCad's own `kicad-python` (MIT), are generated from them. To update them, copy the same files from a newer KiCad tag and update the tag above.
