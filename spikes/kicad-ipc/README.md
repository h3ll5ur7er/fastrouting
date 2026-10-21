# KiCad IPC spike (TRX-17)

A throwaway prototype, outside the Cargo workspace, that proves traxide can talk to a running KiCad 10 from a native process with no Python and no C library: it connects to KiCad's IPC API, reads the open board's layers, nets, pads, a pad's copper polygon and a net class, then adds a track and a via in one commit; with `--undo` it plays Ctrl+Z and checks that one undo removes both.

Findings, sources and what this means for [[TRX-31]] and [[TRX-33]]: [[Research/KiCad-integration]].

```bash
# Against a KiCad you run yourself (Preferences → Plugins → Enable KiCad API):
cargo run --release -- --undo

# Without a person: KiCad 10 in Docker on a virtual display (Linux, needs xvfb-run):
./run-headless.sh path/to/board.kicad_pcb --undo

# The PCM package it was installed from (Linux, needs ImageMagick's convert):
./package.sh
```

- `src/nng.rs`: the client half of NNG's REQ/REP over IPC, about 100 lines.
- `src/messages.rs`: the messages used, defined from field numbers rather than generated from KiCad's `.proto` files.
- `src/client.rs`: the envelope, the token, retrying while KiCad is busy.
- `package/`: `metadata.json` and `plugin.json` of the exec plugin, and `probe.sh`, which records what KiCad passes to an action.
