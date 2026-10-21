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

- [x] Instrument first: per round, connections routed, rip-ups and the places of collisions; a picture of where Issue039's and Mars-64's connections take turns. `cargo run --release -p xtask -- negotiation BOARD` ([[Development#benchmarking]]); the router publishes each rip-up, with the place where the two routes meet. What it shows today (a round being as many commits as there are connections):
  - Issue039 (204): 146 routed after round 1, then 146, 134, 139 and 140 after rounds 2–5 with about 200 rip-ups each, and 157 in the last round, once rip-up has stopped and the queue routes without crossing anything. Past the first round, rip-up there only reshuffles. The rip-ups spread thinly over U1 and the resistor arrays RN1–RN4 and CA1–CA2 (at most 6 in any square millimetre); the pairs that trade places most are neighbouring pins (/VOUT4_DAC and /VOUT5_DAC at U1-21 and U1-20, five times).
  - Mars-64 (106): 57, 65, 67, 67, 66 and 63 routed after rounds 1–6 with about 100 rip-ups each, 83 in the last round. The pairs that trade places are again neighbouring pins of U101 and U102 (SPI_CLK and SPI_MOSI at pins 19 and 18).
  - keyboard-tkl (315): complete in two rounds, 47 rip-ups, all around the RP2350 (U1), with COL15 and COL16 at U1-12 and U1-13 among the pairs.
  - So the time goes to neighbouring fine-pitch pins taking each other's way out, over and over; the connections that only fit once nobody may rip up are the ones that come last. Both point at escaping the pins first.
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

No variant beats rip-up without losing a complete board or taking far longer, so none was kept. The code of all of them is in [[docs/Research/experiments/trx-56-rounds.patch]] (applies to the commit that recorded it; `TRAXIDE_ROUNDS=1` switches the rounds on, `TRAXIDE_LOCKED=1` finishes as below). Rounds settle congested boards much better (Issue039, RelayModule, fewer vias everywhere), but the last few overlaps never resolve, and the routes taken out then no longer fit; rip-up afterwards finishes them on easy boards but churns without end on hard ones. Worth trying next: the overlaps of the last rounds by place rather than by count (they may sit at a few pins, where escaping fine-pitch pins first would help), and finishing with rip-up only for the connections that were taken out, the rest locked.

Tried next, after [[TRX-60]]: rounds, then rip-up and reroute for the leftovers only, every route the rounds settled locked (connections routed, time; same machine, 300 s limit):

| Board | Rip-up (today) | Rounds, then the leftovers ripping up only each other |
| --- | --- | --- |
| keyboard-tkl (315) | **315**, 1.5 s | 314, 4.0 s |
| matrix-6x17 (306) | **306**, 0.2 s | **306**, 0.2 s |
| Issue034 (147) | **147**, 28 s | 146, 12 s |
| Issue015 (61) | **61**, 19 s, 130 vias | **61**, 38 s, 121 vias |
| Issue110-RelayModule (88) | 84, 91 s | **87**, 24 s, 79 vias instead of 108 |
| Issue039 (204) | **157**, 72 s | over 300 s |
| Issue102 Mars-64 (106) | 83, 39 s | **87**, 110 s |
| Issue283 (200) | 182, 93 s, 396 vias | **188**, 84 s, 305 vias |
| Issue029 (91) | **87**, 167 s | 86, 203 s |

Rejected for the same reason as the others: the keyboard and Issue034 lose the one connection whose route the rounds took out last, which no longer fits between locked routes; on Issue039 the leftovers churn. Every rounds variant so far loses exactly that one connection on those two boards, and both times it ends at a fine-pitch pin: COL15 from SW16-1 to U1-12, a pin of the keyboard's RP2350 (QFN), and /M or /N from U4 to U5 on Issue034. Rounds and escaping fine-pitch pins first may well need each other: with the pins escaped before negotiation, nothing is left to take out at a pin.

**Depends on:** [[TRX-54]], [[TRX-55]]: both remove failures that otherwise look like congestion.
