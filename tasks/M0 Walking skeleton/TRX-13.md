---
id: TRX-13
title: Run the route use case in the Routing Manager
type: feature
status: done
priority: high
labels: [manager]
estimate: 3
layer: manager
---

UC1 of [[Architecture/Use-cases#uc1-route-a-board-from-dsn-to-ses]]: load, compile, build, route, store, with events.

- Workflow as data: when profiles differ in their stages (YAGNI); for now load → compile → layout → plan → route → verify → store
- [x] Progress events and cancellation between connections
- [x] Integration test on a 2-layer board from DSN to SES

**Depends on:** [[TRX-7]], [[TRX-8]], [[TRX-9]], [[TRX-10]], [[TRX-11]], [[TRX-12]], [[TRX-6]]
