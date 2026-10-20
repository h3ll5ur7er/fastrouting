---
id: TRX-59
title: Move the board's unlocked wiring when it blocks a connection
type: feature
status: backlog
priority: low
labels: [detailed-routing, board-access]
estimate: 3
layer: engine
---

traxide keeps all copper a board comes with as it is. Specctra marks wiring the router must keep as `(type protect)` or `(type fix)`; `(type route)` wiring is ordinary routing that a router may rip up and route again, and KiCad exports unlocked tracks so. Ten corpus boards carry such wiring (Issue069-KiCad6, 113, 153, 157, 190, 214, 269-caniot, 269-z10, 367 ×2). On two of them it blocks connections that cannot be routed otherwise ([[Corpus-analysis]]):

- Issue269-caniot-tiny-arm: routing has one layer (B.Cu is a power layer, [[TRX-57]]), and the board's 335 tracks enclose J8-9 and the QFP's pins 51–53, so that no way exists at any grid: 5 connections.
- Issue157-TeamAdapt-LinePCB: a track of Net-(D49-Pad1) runs across the pads Q1-3 and Q2-3 of other nets, a rule the board already breaks, and leaves them no way out: 2 connections.

Moving a designer's tracks is a decision for the designer, so it is opt-in.

- [ ] Board Access keeps the wire type: protected (`protect`, `fix`) or not (`route` and others)
- [ ] With an option (`traxide route --reroute-existing`, also in [[TRX-34]]'s Freerouting mode if it has an equivalent), unprotected wiring joins rip-up and reroute as routed copper of connections planned like any other
- [ ] Tests: an unprotected track that blocks the only way is moved; a protected one is not
- [ ] Issue269-caniot-tiny-arm and Issue157-TeamAdapt-LinePCB before → after
