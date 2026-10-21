---
id: TRX-53
title: Route the open board from KiCad with a Python action plugin
type: feature
status: done
priority: high
labels: [kicad, client]
estimate: 1
layer: client
---

UC2 of [[Architecture/Use-cases#uc2-route-the-board-open-in-kicad]] now, for KiCad 9 and 10, until the native plugin of [[TRX-33]] replaces it: a SWIG action plugin that exports the board as DSN, runs traxide and imports the session, in one step.

- [x] `kicad-plugin/plugins`: the action, and the routing kept apart from it so that it runs without KiCad's window
- [x] Finds traxide in `$TRAXIDE`, the plugin's `bin` folder, or the `PATH`
- [x] Says why routing did not happen, and how complete it is when it did
- [x] `cargo xtask e2e` routes each board through the plugin inside KiCad's image and judges it with KiCad's DRC
- [x] PCM `metadata.json`, and a package per platform with its traxide from the Binaries workflow ([[TRX-35]])

SWIG scripting goes away in KiCad 11 ([[Research/KiCad-integration]]), which is why the native plugin stays the goal.

Retired by [[TRX-33]]: the native plugin covers KiCad 9 and 10, so the packages carry it instead and the Python plugin is gone from `kicad-plugin/`.
