---
id: TRX-57
title: Route on KiCad power layers that hold no plane
type: improvement
status: done
priority: medium
labels: [board-access, kicad]
estimate: 1
layer: access
---

The DSN reader treats every layer of `(type power)` as a plane that no track may use ([[crates/access/board/src/specctra/dsn.rs]], `DsnReader::new`). KiCad writes `power` for a copper layer the designer set to the "power" type, but KiCad itself routes and checks tracks on such layers like on any other. On Issue269-caniot-tiny-arm, a 2-layer board whose B.Cu is of type power, without any `(plane …)` on it, 46 of 89 connections stay unrouted because one of their pads, an SMD part on the bottom, has copper only there ([[Corpus-analysis]]).

Measured on a copy of the board with B.Cu as `signal`: 48 of 89 connections instead of 3, clean; with [[TRX-54]]'s case simulated by a 0.05 mm grid, 70 of 89; with a whole-board window as well ([[TRX-55]]), 75 of 89.

- [x] Failing test first: a KiCad DSN with a power layer without a plane takes tracks
- [x] For boards from KiCad: a power layer that no `(plane …)` covers is routable; one with planes stays a plane layer (Issue269-z10_module's In1.Cu and In2.Cu, see [[TRX-58]]); other tools' power layers stay planes
- [x] Say in the route report which layers are kept free of tracks, and why (as planes)
- [x] Issue269-caniot-tiny-arm: 3 of 89 connections routed before, 48 after (0.17 s instead of 12 s), clean
