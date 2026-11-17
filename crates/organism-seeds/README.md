# Development organism seeds (OCEE-0)

This crate provides **explicit, researcher-authored VM programs**. It is not a library of spontaneously evolving organisms.

| Seed | Function | Prior supplied by researcher |
|---|---|---|
| S0 | Read public HELLO, send example, observe/persist response | Public paths, JSON extraction, polling |
| S1 P0 | Probe unknown REMOTE_EVAL opcode via diagnostics | P0 XOR frame layout, opcode range, `BadBody` selector |
| S5 auxiliary | Search among Len/XorFold/ByteAt via two public I/O examples; create and apply tool to held-out input | Preseeded finite vocabulary, one-node skeleton, bounded enumeration, supervised comparator; **not E001 protocol adaptation** |
| S4 P0 | Error-driven one-node graph rewrite; validate and register only a functional executable | Pre-seeded Len/XorFold alternatives, P0 framing prior and two-try selection policy |
| S3 P0 | Derive content-addressed frame-builder executable from discovered opcode; self-test, register and use it | **Pre-seeded P0 frame-building AST fragments**, discovery/selection algorithm, fixed test workload |
| S2 P0 | Use S1 discovery for real FOLD32, discover RESULT, persist value | P0 framing, fixed 3-value FOLD32 encoding, response status/layout |

All private opcode *values*, hidden protocol seeds, task values, and correct outputs are absent from the initial VM programs. That does not make the algorithms themselves emergent: each source module records its built-in priors.

In the older S1->S2 study the researcher installs S2. In the newer **S3** study, one initial S1 variant registers and uses a new *executable tool version* without a researcher handoff. However, the algorithm is specialized from researcher-supplied AST fragments; it is **not** an independently invented algorithm. Experiments may only credit new observations and unknown parameter values to the organism. S4 proves bounded single-node structural repair, not unlimited novel algorithm search. There is still no evidence of cross-family adaptation or open-ended evolution.

To inspect actual content hashes and priors:

```sh
cargo run --quiet -p ocee-organism-seeds --example manifest
cargo test --workspace
```

Evidence: [S0/S1 study](../../experiments/E001/DEV-S0-S1.md) [S2 study](../../experiments/E001/DEV-S2.md) [S3 tool-specialization study](../../experiments/E001/DEV-S3-SPECIALIZATION.md), and [S4 structural repair](../../experiments/E001/DEV-S4-STRUCTURAL-REPAIR.md).

S5 auxiliary evidence: [development operator-search record](../../experiments/E001/DEV-S5-OPERATOR-SEARCH.md). Its per-seed target rule is hidden behind public input/output examples, but the entire candidate vocabulary is a human seed; it is not a newly invented algorithm.
