# TokenBoard report: traxide on a real DSN round trip

In October 2026 the agent that designs TokenBoard (the maintainer's 88-key keyboard, [[TRX-64]]) routed it with traxide 0.1.0 through KiCad 10's Specctra export and import, as its board script does with Freerouting, and reported what KiCad's DRC found. The report and its files (the DSN as exported, traxide's session, KiCad's DRC report, a Freerouting session for comparison) are the maintainer's, not stored here.

**The round trip.** KiCad 10 exports the DSN (`pcbnew.ExportSpecctraDSN`). The board script removes the F.Cu and B.Cu GND pours, fixes 655 wires and 439 vias of pre-routing (`(type fix)`; the RP2350's fan-out among them) and grows the LED cut-out keepouts by 70 µm. traxide routes the DSN, and KiCad imports its session (`pcbnew.ImportSpecctraSES`), refills the zones and runs `kicad-cli pcb drc --schematic-parity`.

**The result.** All 392 connections in 107 s, where Freerouting 1.9.0 takes 25–45 min and leaves 2–6 open; nothing unconnected, no parity issue. But 85 DRC errors and 210 hole warnings that Freerouting's session does not have:

| Finding | Task |
| --- | --- |
| `window` in a plane: traxide refuses the DSN as exported (`unsupported shape window`) | [[TRX-68]] |
| Wires 0–3.5 µm from the LED cut-out keepouts, where the DSN asks 150 µm: 85 errors | [[TRX-69]] |
| The fixed wiring written back as plain routes: KiCad keeps the locked pre-routing and adds the copies, 199 `holes_co_located` | [[TRX-70]] |
| Same-net vias closer than 0.25 mm, some on top of each other, five exact duplicates: 11 `hole_to_hole` | [[TRX-71]] |
| 547 new vias against Freerouting's 194, many in down-and-back-up pairs | [[TRX-72]] |
| 20 fixed fan-out lanes joined elsewhere than at their free end, left dangling | [[TRX-73]] |

What this route through Specctra shows that the corpus's KiCad projects ([[TRX-64]]) do not: the DSN carries the cutouts as keepouts and the pre-routing as fixed wiring, and the session goes back into KiCad by its import, which keeps what is locked.
