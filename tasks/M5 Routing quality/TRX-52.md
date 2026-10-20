---
id: TRX-52
title: Leave a pad from anywhere on its copper
type: improvement
status: done
priority: medium
labels: [detailed-routing]
estimate: 2
layer: engine
---

A track can only start at a pad's centre, the position of its pin. That fails for pads whose centre lies off their copper or off the board, both met in [[TRX-15]]'s corpus:

- edge connectors, whose pads hang over the board's edge: the centre is outside the board (Issue110-testPCBSpecctraFile, XP1-2);
- custom pads such as interdigitated sensor forks, whose pin origin sits in a corner of fingers that the other pin's fingers surround (Issue069-TestSensel).

- [x] Where the way from a node to the pad's centre is blocked, a way to the nearest point of the pad's copper; the path then begins and ends there
- [x] Test for an edge connector; the fork pads of Issue069-TestSensel route 5 of 6 connections instead of 2
