---
id: TRX-56
title: Negotiate congestion until it settles
type: improvement
status: in-progress
priority: high
labels: [detailed-routing]
estimate: 4
layer: engine
---

(Measured with negotiation ending after two rounds without gain; it now ends after four, which gives Mars-64 back 71 of 94 nets.) 131 connections on 9 corpus boards route alone but not together with the rest ([[Corpus-analysis#congestion]]); a wider window ([[TRX-55]]) settles 10 of them, the rest are lost in negotiation. Rip-up and reroute in `Router::route_plan` ([[crates/engines/detailed/src/lib.rs]]) stops ripping up once 2 × (number of connections) rip-ups have passed without more connections routed (`STALLED_ROUNDS`, from [[TRX-48]]), and then the connections still queued search without crossing anything and fail. On every congested board that rule ends negotiation: Issue039 after 472 rip-ups for 204 connections, Mars-64 after 412 for 106. No profile setting helps: `max_rip_ups` 100, `history_cost` 0 or 5, `heuristic_weight` 1.0, `via_cost` 0.5, `batch_size` 1 or a 0.05 mm grid route at most 9 more connections on Issue039 (155 → 164 of 204) and 3 more on Mars-64 (72 → 75 of 106).

Freerouting 2.5.0, run as a black box with 900 s, does much better: Mars-64 (Issue326) with all 94 nets against traxide's 62 (its only violations are 6 copper-to-edge ones), Issue039 with 64 of 74 nets against 32 (6 clearance violations), Issue283 with 55 of 58 against 42 (17 edge violations). Its log shows a fan-out stage that escapes the SMD pins first (236 of 242 on Issue039), then whole passes over all unrouted items with rising rip-up costs.

The routes traxide leaves on these boards wander and use many vias (Mars-64: 199 vias for 72 connections; Freerouting 155 for all of them), taking room that the last connections need.

- [ ] Instrument first: per round, connections routed, rip-ups and the places of collisions; a picture of where Issue039's and Mars-64's connections take turns
- [ ] Negotiate in rounds over all connections, PathFinder's way: while negotiating, overlaps are allowed at a present-congestion price that rises each round, history costs accumulate, and only the final round must be legal; compare with the current first-come rip-up on the corpus
- [ ] A stopping rule from progress across rounds and the time budget, not a fixed count of rip-ups
- [ ] Try escaping fine-pitch SMD pins first (a short stub and a via, planned before the long connections, as [[TRX-24]] plans hemmed-in pads first) and measure it separately
- [ ] Corpus, unrouted connections before → after: Issue039-bug-design (49), Issue102 and Issue326 Mars-64 (23 each, with [[TRX-54]]'s 11), Issue283-UnconnectedTracesUnderPads (22), Issue508-DAC2020_bm01 (3), Issue269-caniot-tiny-arm (1); the keyboard and the key matrices as before ([[Benchmarks]])

Tried in October 2026, on top of [[TRX-55]]: negotiation in rounds (every connection routed each round, crossing others at a present price for each connection crossed, from `rip_up_cost` up by half each round, history recorded where they overlap, only the overlapping ones routed again; ending when four rounds bring no fewer overlaps, then the most overlapping routes taken out until the rest is legal), in three ways of finishing (connections routed, measured on this machine):

| Board | Rip-up (today) | Rounds, then the leftovers routed strictly | Rounds, then rip-up | Rounds, then rip-up within a budget of the leftovers |
| --- | --- | --- | --- | --- |
| keyboard-tkl (315) | **315**, 1.4 s | 314, 3.2 s | **315**, 4.5 s | 314, 3.8 s |
| Issue034 (147) | **147**, 23 s | 146, 11 s | **147**, 24 s | 146, 11 s |
| Issue039 (204) | 157, 63 s | **180**, 153 s (53 of 74 nets instead of 37) | over 300 s | 158, 177 s |
| Issue102 Mars-64 (106) | 81, 90 s | **85**, 60 s, 200 vias instead of 284 | over 300 s | 80, 81 s |
| Issue110-RelayModule (88) | 77, 68 s | 86, 27 s | **88**, 64 s | 86, 36 s |
| Issue015 (61) | **61**, 18 s, 130 vias | 61, 33 s, 121 vias | 61, 36 s | 61, 35 s |
| Issue029 (91) | 87, 149 s | | over 300 s | 83, 116 s |

No variant beats rip-up without losing a complete board or taking far longer, so none was kept. Rounds settle congested boards much better (Issue039, RelayModule, fewer vias everywhere), but the last few overlaps never resolve, and the routes taken out then no longer fit; rip-up afterwards finishes them on easy boards but churns without end on hard ones. Worth trying next: the overlaps of the last rounds by place rather than by count (they may sit at a few pins, where escaping fine-pitch pins first would help), and finishing with rip-up only for the connections that were taken out, the rest locked.

**Depends on:** [[TRX-54]], [[TRX-55]]: both remove failures that otherwise look like congestion.
