# OCEE-0 Rust world kernel — P1/P2/P3 integration slices

This crate implements the deterministic **world host** for OCEE-0. The first P1 core and a P2 **program lifecycle bridge** are running. It is not yet a complete ecosystem, an evolving digital organism, or an E001 result.

## Implemented and CI-verified

- World tick scheduling: due events/messages → epoch grants → at most one external action → audit record.
- A/B/C host topology, local objects/CAS, quota rejection, delayed C inbox, hash-chained audit and JSON snapshot/restore.
- Typed program versions stored at `/self/versions/<SHA256>` with canonical byte validation and **immutable versions**.
- Atomically selected active version via CAS `WRITE /self/active`, with historical rollback possible.
- `tick_autonomous(actor)` resumes the program VM, bills its microsteps and processes at most one yielded world action; action feedback is delivered on the following tick.
- Isolated `EXEC(program_hash,input)` for **pure candidate computation or read-only local snapshot tests**, with metered input-dependent VM execution. A yielded world side effect is rejected and billed.
- End-to-end engineering fixture showing a program constructing a content-addressed candidate, writing it via world calls, selecting it and executing the adopted version. **The candidate is prearranged in the fixture, not spontaneously invented.**
- Optional isolated simulated protocol C/B service: P0/P1/P2 decoder, B's bounded FOLD32 execution and gas billing, delayed response queue and replayable service state.
- Legal protocol HELLO frame pair published through a read-only world object `/public/C/hello` every 256 ticks for future learning experiments.

## Intentionally deferred

- Full v0.2 program ISA (cross-function `CALL`, `Unit`, `Result<T>`); supported `RepeatBounded` is already bounded and metered. A caller-configurable EXEC gas limit is not yet implemented.
- Full S/O initial agents, genuinely novel program algorithm synthesis beyond seeded templates, and realistic adaptive repair/rollback strategy.
- Full E001 workload scheduler, A/B/C/D controls, protocol-general adaptation, calibration and E001 confirmatory study. P0/P1/P2 simulated services exist.
- SQLite atomic transactions, finalized cost schedule, durable cross-version state encoding, fully generalized capability security.

The current action driver and initial program-selection fixture remain available for testing. No tests should be described as evidence that a life form or self-directed innovation has appeared.

### S10 development-only active semantic witness

After an isolated `EXEC` of the **currently registered tool** on the
researcher-chosen empty input, the world publishes
`/public/E001/semantic-witness` even when the tool returns `Ok`. S10
compares the old tool's actual numeric result against that fixture and can
detect a silent wrong answer. The report is NOT emitted on a gas-denied,
non-executed action. It models simulated correlated supervisory feedback:
it is neither an independent source of truth nor a general correctness oracle
for arbitrarily chosen future inputs.

### S9 development-only post-fault external witness

When a **registered** transform tool fails an isolated empty-input `EXEC`, the
world publishes read-only `/public/E001/boundary-witness`. Its two labels can
agree on 0 or 1, be absent, or conflict depending on the development seed.
No witness exists before the failure. This deterministic supervised fixture
models environmental feedback, not independent trustworthy observers; it
provides no general semantic-error detector. A fault-free tool producing a
wrong output may never trigger the witness (S9 seed 17).

### S5/S6 development-only learning fixtures

Read-only `/public/E001/transform` and `/public/E001/compose` objects presents two labeled byte-transform examples and one input without an answer. A hidden rule selected from three existing VM primitives is used **only** for researcher-side evaluation. S5 tests selection among single-op programs; S6 additionally tests a bounded grammar of two unary nodes and one binary node. Neither is part of E001's official protocol families or scoring design.

## Run

```sh
cargo test --workspace
```

Core behavior contract: [04-ENGINEERING-CONTRACTS.md](../../04-ENGINEERING-CONTRACTS.md). Current checkpoint: [07-ENGINEERING-CHECKPOINT.md](../../07-ENGINEERING-CHECKPOINT.md).
