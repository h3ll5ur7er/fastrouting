---
id: TRX-17
title: "Spike: KiCad IPC API from a native exec plugin"
type: research
status: todo
priority: medium
labels: [kicad, spike]
estimate: 1.5
---

Prove the in-editor path before building it ([[Research/KiCad-integration]]).

- [ ] A Rust `exec` plugin that reads nets, stackup and pad polygons through `kicad-ipc-rs` on KiCad 10
- [ ] Create a track and a via in one commit; check one undo removes both
- [ ] Install through a local PCM repository on Windows, macOS and Linux; check the toolbar icon and macOS quarantine
- [ ] Note what KiCad 11 (master) changes: design rules over IPC, headless mode
- [ ] Settle the licence question: KiCad's `.proto` files are GPL-3.0, `kicad-ipc-rs` (generated from them) is MIT. Ask the KiCad developers or check their API licensing statement; if in doubt, define our own messages from the protocol documentation, without copying the `.proto` files

**Depends on:** [[TRX-1]]
