# OCEE-0 P1 — Deterministic World Kernel (first implementation slice)

Implements a **deterministic, externally driven** world state transition system using Rust. This is the first engineering slice, **not** a completed OCEE-0, learning organism, protocol service, or scientific experiment.

Covered:
- Unified tick: due events and messages → epoch grant → one externally supplied action → state/audit commit.
- Deterministic gas, storage and C-link bandwidth accounting; resource refusal without implicit "death".
- Node A local actor, C message endpoint, B direct-access denial; delayed message delivery.
- Local versioned objects with CAS, atomic quota checks, and stable ordered maps.
- JSON snapshots, SHA-256 state digests and tamper-evident audit record chain.
- Five focused tests for timing, permissions/messages, storage/CAS, replay, and gas exhaustion.

Deliberately **not implemented**:
- Activity VM/continuations, real organism autonomy or self-modification (P2).
- C/B protocol service, unknown-protocol P0/P1/P2, workload and reference solver (P3).
- SQLite transaction durability, multi-actor capabilities, general host security (later P1 hardening).
- E001 calibration and confirmatory experiments (G1/G2 remain NO-GO).

### Run

```bash
cargo test --manifest-path crates/world-kernel/Cargo.toml
```

The P1 runner feeds actions externally. An `Exec` request returns `Unsupported`, rather than pretending the unimplemented VM exists. Failures in this prototype cost zero gas; this cost policy is **provisional** and must be calibrated/versioned before E001. `serde_json` serialization is stable within this version's explicit struct/BTreeMap layout; canonical inter-version state encoding is not yet frozen.

Specification: [04-ENGINEERING-CONTRACTS.md](../../04-ENGINEERING-CONTRACTS.md). 
