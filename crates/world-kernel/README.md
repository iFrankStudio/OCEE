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

### S13-A organism-created challenge echo (development-only)

When a registered tool's `EXEC` actually occurs, the public simulated
reports optionally echo the organism-owned `/self/repair/probe-nonce` as
`probe_challenge`. The value is based on public `Observe` state and
is NOT confidential, unpredictable or authenticated. The separate
[S13-A VM](../../experiments/E001/DEV-S13-CHALLENGE.md) checks two echoes
against the stored current challenge, identifying unmodified old-run
reports with the same subject program. It does not stop coordinated forgery.

### S13 exact EXEC receipt (development-only)

The kernel returns optional `ActionResult.exec_step` on Ok/InvalidCode
isolated EXEC attempts, and not on Gas-refused/non-EXEC actions. S13's active
VM stores this response and matches both simulated reports'
`observed_after_step` to the particular tool call, rejecting old
same-program/same-input reports. The receipt is U32-scoped in this
experimental implementation and **not** a cryptographic authenticity or
cross-world trust mechanism. See [S13 study](../../experiments/E001/DEV-S13-EXEC-RECEIPT.md).

### S12 bound evidence metadata (development-only)

Simulated witness objects now include `source_id` (declared producer role)
and `subject_program_hash` (the exact program hash whose registered
`EXEC` produced the report), alongside task, input and step. An S12
organism checks both metadata fields before trusting either channel.
Neither field is cryptographically authenticated; a privileged attacker
can still replace all values or replay old evidence for the same version.
See [S12 experiment](../../experiments/E001/DEV-S12-PROVENANCE.md).

### S11 second simulated semantic check

After an actually executed registered-tool empty-input `EXEC`, the world
publishes `/public/E001/semantic-audit` with a separate implementation
of the same researcher-defined empty-input reference. It is world-owned,
read-only for the organism, and withheld on gas-refused actions.
S11 compares it with the primary semantic witness before adopting a
tool repair. **Two code implementations in one simulator are not genuinely
independent sources**: simultaneous falsification still causes wrongful
adoption. See [S11 study](../../experiments/E001/DEV-S11-CORROBORATION.md).

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
