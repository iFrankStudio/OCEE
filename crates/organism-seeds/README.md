# Development organism seeds (OCEE-0)

This crate provides **explicit, researcher-authored VM programs**. It is not a library of spontaneously evolving organisms.

| Seed | Function | Prior supplied by researcher |
|---|---|---|
| S0 | Read public HELLO, send example, observe/persist response | Public paths, JSON extraction, polling |
| S1 P0 | Probe unknown REMOTE_EVAL opcode via diagnostics | P0 XOR frame layout, opcode range, `BadBody` selector |
| S10 auxiliary | Proactively execute an adopted tool on the seeded empty probe; compare even successful outputs against a simulated semantic witness; repair or abstain | Researcher-selected input, simulated correlated evaluator, finite repair candidates and guarded adoption; **not general semantic-error discovery** |
| S9 auxiliary | Read simulated post-fault public witness, compare two boundary observations, choose guard candidate or explicitly abstain | Fully pre-seeded repair vocabulary, fixture-provided witness correctness, trust in two correlated labels, fault-triggered observation strategy |
| S8 auxiliary | Generate two guard variants, reject the executable-but-wrong first proposal, validate and CAS-adopt second | Researcher-seeded [1,0] vocabulary, zero-output oracle, branch skeleton, bounded search strategy |
| S7 auxiliary | Discover tool's out-of-training empty-input fault; insert conditional graph guard; retest; CAS update tool while retaining old version | **Seeded empty→0 hypothesis**, guard AST, probe strategy, S6 finite grammar; not a general autonomous diagnosis algorithm |
| S6 auxiliary | Finite 3×3×2 multi-node operator composition, trained on labeled I/O examples and validated on held-out input | Human-seeded operator grammar, skeleton, candidate enumeration and feedback selection; not P1/P2 discovery |
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

S6 multi-node composition evidence: [development study](../../experiments/E001/DEV-S6-COMPOSITION.md). Its search grammar was explicitly designed; no novel primitive or search-policy origin is claimed.

S7 in-world repair evidence: [boundary-repair study](../../experiments/E001/DEV-S7-BOUNDARY-REPAIR.md). The tool's faulty behavior and selection of a surviving program version are genuinely observed, but the guard algorithm was provided in the initial seed.

S8 measured selection under the original A budget: [bounded repair study](../../experiments/E001/DEV-S8-REPAIR-SEARCH.md). The correct target is supplied as an initial prior, not inferred from the failure alone.

[S9 witness study](../../experiments/E001/DEV-S9-BOUNDARY-WITNESS.md) records both target values and explicit abstention. Its negative seed 17 proves a fault-triggered witness cannot find an output error when the tool executes without a runtime fault. Two witness labels in this fixture are not independent sources.

[S10 study](../../experiments/E001/DEV-S10-SEMANTIC-PROBE.md): an Ok-returning but incorrect tool can now be challenged and repaired on a seeded probe. Two same-source labels remain vulnerable to consistent false evidence, as S9 already demonstrated.
