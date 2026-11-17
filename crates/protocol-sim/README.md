# OCEE-0 P3: deterministic protocol-sim

Research-side, **offline**, deterministic simulation of three previously specified protocol families. Built to create a learnable environment, not to supply an agent with a protocol parser.

- **P0:** marker, kind, opcode, LE length, bytes, XOR check.
- **P1:** 0x7E delimited / 0x7D escaped inner frame, BE length and seeded check key.
- **P2:** nested TLV (seeded tags and order), with canonical tag-order XOR check.
- B accepts only a bounded `FOLD32` data workload (1–64 `u32` values).
- C supports HELLO, CAPS, REMOTE_EVAL and RESULT with an actor/request ID namespace, idempotent replay and delayed result readiness.
- Malformed frames return bounded diagnostics instead of protocol secrets.
- A versioned fixed SplitMix64 generator produces reproducible **non-cryptographic** protocol parameters; it is not a security mechanism.

Only the world host imports `Protocol` and `Service`; organism VM code cannot call the generator. Reference request builders in tests are research fixtures, never agent knowledge.

## Status

The crate's frame/semantic tests, P0/P1 limited-feedback opcode probe, and its integration with `world-kernel` pass CI. The opcode probe starts with a researcher-defined generic P0/P1 framing model; it does not infer an unseen grammar or exercise a VM organism. The host publishes legal HELLO request/response samples to `/public/C/hello` for future protocol-discovery research. There is **no independent restricted learner yet**, so protocol learnability and E001 success are unverified.

Run: `cargo test --workspace`.
