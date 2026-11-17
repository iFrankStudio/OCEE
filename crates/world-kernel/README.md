# OCEE-0 Rust world kernel — P1 + P2 integration slice

This crate implements the deterministic **world host** for OCEE-0. The first P1 core and a P2 **program lifecycle bridge** are running. It is not yet a complete ecosystem, an evolving digital organism, or an E001 result.

## Implemented and CI-verified

- World tick scheduling: due events/messages → epoch grants → at most one external action → audit record.
- A/B/C host topology, local objects/CAS, quota rejection, delayed C inbox, hash-chained audit and JSON snapshot/restore.
- Typed program versions stored at `/self/versions/<SHA256>` with canonical byte validation and **immutable versions**.
- Atomically selected active version via CAS `WRITE /self/active`, with historical rollback possible.
- `tick_autonomous(actor)` resumes the program VM, bills its microsteps and processes at most one yielded world action; action feedback is delivered on the following tick.
- Isolated `EXEC` for **pure candidate computation**. A yielded world side effect is rejected and billed.
- End-to-end engineering fixture showing a program constructing a content-addressed candidate, writing it via world calls, selecting it and executing the adopted version. **The candidate is prearranged in the fixture, not spontaneously invented.**

## Intentionally deferred

- Full v0.2 program ISA (`CALL`, `REPEAT_BOUNDED`, `Unit`, `Result<T>`), general authorized snapshot reads during isolated EXEC.
- S/O initial agents, genuine autonomous candidate search/feedback selection, tool composition, realistic repair/rollback strategy.
- Protocol C/B service, P0/P1/P2 hidden protocol, workload, four groups, calibration and E001 confirmatory study.
- SQLite atomic transactions, finalized cost schedule, durable cross-version state encoding, fully generalized capability security.

The current action driver and initial program-selection fixture remain available for testing. No tests should be described as evidence that a life form or self-directed innovation has appeared.

## Run

```sh
cargo test --workspace
```

Core behavior contract: [04-ENGINEERING-CONTRACTS.md](../../04-ENGINEERING-CONTRACTS.md). Current checkpoint: [README.md](../../README.md).
