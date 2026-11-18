# S18-R — active P1 VM mid-scan snapshot/restart (development regression)

Date: 2026-11-18. This is **not** E001 confirmation; G1/G2 remain NO-GO.

## Hypothesis and intervention

S18's active VM performs a bounded P1 opcode scan using persisted VM continuation, real A→C messages and delayed responses. A snapshot during the scan must not reset candidate progress or duplicate B work.

The focused test `crates/organism-seeds/tests/s18_restart.rs` starts the existing seed-42 P1 activity under the standard 12,000 A gas/epoch quota, executes ten autonomous ticks, snapshots the still-active world, and independently continues both the original and deserialized world. It compares terminal state hashes, checks the final successful VM return, and asserts exactly 20 B gas. This tests a **host-initiated restart of persisted state**, not self-directed migration or autonomous repair. The initial program and P1 grammar are unchanged.

Code commit: [2e049e3c](https://github.com/iFrankStudio/OCEE/commit/2e049e3c152489197d8768cfd3340c544e6e45fd).

## Evidence status and limits

At authoring time the GitHub Actions list still showed the prior successful [#44313014356](https://github.com/iFrankStudio/OCEE/actions/runs/44313014356) (81 workspace tests) and did **not** yet list the new test commit. Thus the new test is **submitted, not CI-confirmed**. Do not report this as a passing restart experiment until its own completed run is inspected. A failure may expose snapshot/continuation semantics or an invalid test assumption and must be investigated before changing assertions.

This regression does not establish new protocol learning, new tool generation, independent discovery of correctness, or cross-episode transfer. Its role is to verify a prerequisite for long-running active experiments without multiplying full S18 scenarios.

## Next checkpoint

1. Find the Actions run for `2e049e3c` or a later descendant that includes this test; inspect completed status and logs.
2. If failure: reproduce the specific mismatch and fix root cause with one focused test, avoiding unrelated test proliferation.
3. If success: append run ID, exact test result, and any resource/accounting difference here and in the engineering checkpoint.
4. Proceed to a new capability test: persist a discovered *executable* P1 tool and validate it after a fresh episode, with a changed-protocol negative-transfer control. Do not count the researcher-supplied grammar as invented.
