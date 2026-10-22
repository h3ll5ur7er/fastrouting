---
id: TRX-69
title: Keep the clearance from DSN keepouts
type: bug
status: todo
priority: high
labels: [dsn, rules, verification]
layer: engine
---

From the TokenBoard report ([[Research/TokenBoard-report]]): 85 wire segments run 0–3.5 µm from keepouts where the DSN's default rule asks `(clearance 150)`; KiCad's DRC reports them. All are at the LED cut-out keepouts that KiCad exports inside the images of the SK6812MINI-E footprints (`(keepout "" (polygon signal 0 …))`), which are Edge.Cuts cutouts on the board. Freerouting keeps the class clearance from keepouts.

traxide treats a keepout as copper may not overlap it, as KiCad's rule areas ask; a Specctra keepout asks the clearance as well.

- [ ] A failing test: a track beside a DSN keepout keeps the default clearance, in routing and in verification
- [ ] Keepouts carry the clearance copper keeps from them: the DSN's (its clearance rules, by default the class's), none for KiCad's rule areas
- [ ] The report's board: no wire closer than 150 µm to a keepout
