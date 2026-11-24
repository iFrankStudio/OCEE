# DEV S29 — Repeated reversible experience revision via existing own-object CAS

Prior one-way revision #45387874278 3/3, recovery and reversal-cost both verified. Reuse existing /sys/self-object/maintenance/s29-perception-experience (actual atomic owner-only bytes+U32 version, 3A for one byte) rather than a new sensor. Both policies derive from accepted REVISE: thread actual observed version in continuation, recognize exact marker1 or0; all existing-marker writes use that observed version, abstain on reserved U32::MAX. Create only when actual NotFound (existing invalid/non-Ok must not create). Both revoke 1→0 after one real new commit. REVERSIBLE additionally re-enables 0→1 after two real new commits; ONE_WAY does not. The rule is predefined, not learned; no phase/scenario or unseen opportunity information supplied. No kernel, permission or VM changes, no external marker edits. Failed CAS does not change marker; experiment requires actual Ok. No general concurrency claim.

Seed129 P2/B20, common paid install/pins, free3000B, natural grants, equivalent same-root restart each phase, 180tick each. Existing 20/60 limits, all receipts independent output/digest/version0, exact replay, no Gas/maintenance, correct return and real fees. Five fixed tapes, distinct IDs; new g=[6,7,2,1], h=[19,8,6,3,2], i=[20,1,9], j=[7,13,2,4,9].
0 train a/+7 b/+9 a: both two commits, create1/v0.
1 c/+40 d sustained: both only c, revoke0/v1.
2 e/+40 f sustained: both two, REVERSIBLE CAS1→1/v2 (expected_version1, new byte1); ONE_WAY remains0/v1.
3 g/+9 h/+11 g: REVERSIBLE captures h, ONE_WAY misses; both retain prior marker.
4 i/+40 j sustained: REVERSIBLE misses j, revokes using observed v2→0/v3; ONE_WAY captures j, stays0/v1.
Require actual paid version/bytes read each phase and precise CAS precondition/fee/version transition; check every prior receipt and all versions, actual observation before execution, no duplicate or researcher task/experience writes, actual costs/storage/install and exact snapshot/audit/publication replay. Count all costs; compare opportunity tradeoffs, not universal dominance. Preserve any failure and no moving tapes. G0 GO/G1,G2 NO-GO; formal E001 uncalibrated/unpreregistered.


## Compilation-only failure and exact repair
[Run 45389246393](https://github.com/iFrankStudio/OCEE/actions/runs/45389246393) @f045d14 failed to compile E0428, duplicate EXPERIENCE_META and repeated_experience_root. No experimental trajectory or scientific result. The trailing duplicate was verified byte-for-byte equal after trimming whitespace, then removed, retaining original builder and complete frozen test. Repair @ec7e58c, [CI #45390225834](https://github.com/iFrankStudio/OCEE/actions/runs/45390225834). No change to policy, tapes, budgets, world, acceptance or predictions. Original red retained.
