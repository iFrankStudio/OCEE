# VM bounded-loop counter scope: source-confirmed liveness mechanism

Evidence baseline: main `cba46deb2f00524693a08dbabda4ed28001eecf9` (2026-11-21 UTC). This is a **static source finding**, not a new native CI result or E001 result.

## Confirmed implementation fact

In `crates/organism-vm/src/lib.rs`, `Continuation.loop_counters` is a `BTreeMap<String,u32>`. At each `Terminator::RepeatBounded`, the interpreter executes `self.loop_counters.entry(block.id.clone()).or_insert(0)`, increments it while below the bound, and branches to `done` when exhausted. The interpreter does **not** reset this entry on a subsequent entry into the same block; the counters persist across yields and snapshot serialization.

In `crates/organism-seeds/src/s23_p2_transfer.rs`, the shared `s23_receive_retry` block has `limit:128`, `done:"failed"`. `s44_outcome_wait.rs` sends `InFlight`, `CommittedB` and `HandledNoB` back to this same block. Consequently, all visits to this header within one continuation share one cumulative 128-iteration budget, irrespective of candidate opcode or request identity. This is **confirmed by code semantics**. The extent to which it explains the observed high-delay failure remains an empirical hypothesis pending a VM trace.

## Narrow falsifiable test, not yet executed

1. Construct an isolated minimal typed VM graph with one bounded header (`limit:2`) entered twice from distinct candidate states, separated by a successful feedback/yield. Record `loop_counters["header"]` after each visit, and compare snapshot-reloaded continuation. The current implementation should show cumulative consumption rather than a fresh per-candidate allowance.
2. Instrument a **test-only** S44 global-delay=6 run, recording `step`, `continuation.block`, `loop_counters["s23_receive_retry"]`, candidate opcode, actual u64 SEND ID, `RequestOutcomeFor` state, B charges and RESULT delivery. Require a negative control with normal delay, a delayed-ACK-only control, and a forged-reply rejection control. Compare terminal reason and actual response delivery tick. Never infer missing B execution from a missing reply.
3. If cumulative accounting is causally implicated, separate opcode-search attempts from in-flight response waiting using bounded, metered control-flow state, **not** a global reset of `Continuation.loop_counters` (which could defeat termination guarantees). Maintain a hard aggregate search ceiling and explicit per-request wait ceiling, with distinct IDs/counters and snapshot persistence. Do not re-SEND when status is Unknown or CommittedB.
4. Validate the minimal VM counter test, targeted active P2 tests and negative controls in native Rust CI before claiming a liveness improvement. No new native test was run for this note.

Prior native evidence: [S44 3/3 green](https://github.com/iFrankStudio/OCEE/actions/runs/44862964346); [global-delay red counterexample](https://github.com/iFrankStudio/OCEE/actions/runs/44862804649). Neither run establishes a fix. Full workspace [#44861571121](https://github.com/iFrankStudio/OCEE/actions/runs/44861571121) was cancelled at an earlier SHA.

Scientific status is unchanged: no E001 calibration or preregistration, no demonstrated open-ended evolution. This document is diagnostic evidence, not a mandated next Study: future work must re-read main HEAD and choose based on latest evidence.
