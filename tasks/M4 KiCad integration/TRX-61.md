---
id: TRX-61
title: Read the design rules over IPC on KiCad 11
type: feature
status: backlog
priority: low
labels: [kicad, access]
layer: access
---

KiCad 10 serves no design rules over its API, so the `kicad-ipc` adapter reads the board's minimums and custom rules from the project's files ([[TRX-31]], [[Research/KiCad-integration#the-ipc-api]]). KiCad 11 adds `GetBoardDesignRules` and `GetCustomDesignRules`.

- [ ] Once KiCad 11 is released: read the rules over IPC when KiCad serves them, the project's files otherwise
- [ ] The end-to-end test in a running KiCad on KiCad 11 too
