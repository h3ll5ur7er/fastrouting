---
id: TRX-22
title: Run the benchmark corpus and gate releases on it
type: feature
status: done
priority: high
assignee: claude-gate
labels: [benchmark, ci]
estimate: 3
layer: manager
---

UC6 of [[Architecture/Use-cases#uc6-run-the-benchmark-corpus]]: the Inspection Manager queues route jobs and compares.

- [x] A stored baseline of traxide's corpus results per board: `corpus/baseline.json`, written by `cargo xtask corpus` as `target/corpus/results-K-of-N.json`
- [x] A gate that fails the Benchmark workflow on a regression against the baseline and lists the improvements: `cargo xtask gate`, the workflow's `gate` job
- [x] Raise the baseline on purpose: `cargo xtask gate --update-baseline`, or the workflow's `corpus-results` artifact copied over `corpus/baseline.json`
- [x] The per-board table, now against the baseline, in the job summary

**Decisions (the gate, [[xtask/src/gate.rs]]).** The baseline keeps, per board, traxide's outcome (finished, stopped at the limit, failed), nets, nets routed, rules broken and time, and the time limit it ran with; results with another limit are not compared. The gate fails when a board breaks any rule (beyond those the board broke before routing), when a board the baseline completes is not complete, when a board the baseline finishes stops or fails, when a board stopped in the baseline now fails, when a board routes more than 2 % of its nets fewer than the baseline (`nets / 50`, so any loss on a board under 100 nets), and, unless `--partial`, when a board of the baseline is missing from the results (a shard that failed). Tolerances: a board the baseline finishes in at least half the time limit may stop at it (Issue029, Issue145 and Issue219 flip so with the same routes, [[Benchmarks]]); a few nets fewer within the 2 % and a board more than twice and 10 s slower are listed as notes, not failures, as times vary with the runner. More nets, a stopped or failed board that finishes, are improvements; the gate lists them so that the baseline can be raised, but does not fail on them. Only traxide is gated: Freerouting's results vary from run to run and are not ours to keep. The shards still only record; one `gate` job after them compares all boards at once, so the summary shows one table of the 76 boards.

**The first baseline** (commit c2703e7, this container, 120 s per board, three shards at a time): 49 boards complete, 16 incomplete, 8 stopped, 3 failed, none breaking a rule, as CI's 50, 17, 6 and 3 within the boards that sit near the limit. Boards a CI run finishes where the baseline stopped show as improvements; it comes from a local run, not a CI runner, so replace it with the `corpus-results` artifact of the first green Benchmark run on main. The workflow runs on pushes that touch `corpus/**` too, so a commit that only raises the baseline runs the gate. Checked by hand: today's results pass, and a doctored shard (a net lost on a complete board, rules broken, 5 nets fewer, a 0.1 s board stopping) fails with exactly those regressions.

**Left for later, in [[TRX-66]].** The history across commits and the comments on pull requests need a place to keep runs ([[TRX-21]]) and a token that may write comments; the Inspection Manager's `traxide bench` (UC6) and the comparison with the last release's results instead of a stored file come with it. The workflow still runs on every push that touches the code, not only on main.

**Depends on:** [[TRX-15]], [[TRX-20]], [[TRX-21]], [[TRX-14]]
