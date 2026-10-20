---
id: TRX-33
title: Route inside KiCad with a native plugin
type: feature
status: todo
priority: high
labels: [kicad, client]
estimate: 3
layer: client
---

UC2 of [[Architecture/Use-cases#uc2-route-the-board-open-in-kicad]]: an `exec` plugin, no Python.

Facts from [[TRX-17]]: [[Research/KiCad-integration#the-plugin]].

- [ ] `plugin.json` with `runtime.type: "exec"` and actions *Route board*, *Route selected nets*, *Optimise*. Actions get no arguments and run from the plugin folder: one entry point per action (one small file per action or one binary that reads its name from `argv[0]`)
- [ ] The plugin finds the board through the API and the project through `KIPRJMOD`
- [ ] Progress window from events; cancel
- [ ] PCM package per platform: `plugins/` with `plugin.json`, binary and light and dark icons, `resources/icon.png`, `metadata.json` with `platforms` and `kicad_version` 10.0
- [ ] Say in the PCM description and the README that the KiCad API must be enabled first: without it KiCad shows no traxide button at all
- [ ] Install from a local PCM repository on Linux, Windows and macOS: executable bit (kept on Linux), toolbar icons, Windows named pipe, macOS notarisation and quarantine
- [ ] Retire the Python action plugin of [[TRX-53]] once this covers KiCad 10

**Depends on:** [[TRX-31]], [[TRX-13]]
