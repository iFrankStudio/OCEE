# DEV S29 — Actual recovery revises prior negative memory, without confusing task success and memory commit

**Frozen before implementation; native result verified.** Baseline own-task quota override 3/3 #45318054188; independent history-causal transfer #45318793560 still pending at protocol freeze. The current override leaves actual stale negative b1/v0 after success. This study changes only the researcher-authored VM policy's post-success memory update, not kernel authority, task, catalog or physical GC.

## Frozen paired seed129 pilot

Use actual P2 seed129, common v2 paid world, true orphan target, all pins, and two preinstalled/pinned policy roots: STALE (existing quota_task_root(true)) and REVISE (same root except actual successful retry triggers a paid read of dry metadata, validates b0/b1, extracts real version, paid CAS b0). Only genuine GC followed by actual task retry Ok can attempt revision. Immediate initial task Ok does not change maintenance memory. If metadata read or CAS fails, do not falsely convert already committed task into failed task. Return success for completed artifact; memory receipt independently records whether revision succeeded. No retry of memory CAS, no hidden world state or host-chosen candidate. All task/maintenance actions come from VM.

Common initial history: activate original dry root, actual unsuccessful scan/GC creates b1/v0; all roots pinned before scan. After matched external pin CAS releases target (RELEASED) or retains reference (PINNED), capacity used+195; natural epoch grant. Both policies from same snapshot, legal equal-cost activation, task 280B, horizon +90 ticks. For RELEASED expect both actual Quota→GC276B→retry Ok; STALE remains b1/v0, REVISE paid CAS b0/v1 after task. For PINNED both Quota, exhaustive no GC, actual b1/v1 and no false positive. No claim of policy utility dominance. Both complete exact snapshot/audit replay, all live versions safe, actual P2 B20 unchanged, separate task vs VM vs world action A, explicit memory writes and index peak.

Third arm STALE_CAS: identical RELEASED starting snapshot and REVISE policy, compare no-race replay versus a fixed *receipt-bound* concurrent researcher CAS b1/v0→b1/v1 immediately after the VM's paid post-recovery metadata read and before next autonomous tick. The original task has already returned Ok; stale b0 write must get actual Conflict, not overwrite b1/v1 or erase successful task, no second CAS. Exogenous CAS charged separately and same event replay in this arm, not a fixed-time paired lifetime tape. Researcher race is not evidence that an independent organism exists. Frozen 90-step horizon; retain reds; no changes to old successful pilot or pending history study.

This is finite state/receipt-dependent memory revision, not learned policy invention, general adaptation or open-ended evolution. G0 GO/G1,G2 NO-GO, formal E001 uncalibrated/unpreregistered.

## Native evidence — 2026-11-24

[Run 45319202821](https://github.com/iFrankStudio/OCEE/actions/runs/45319202821) at 9681065a1875483a227de5ea0376a60339decd66: **3/3 native jobs passed**, full logs inspected, each one test passed/no failed. Same true seed129 P2 B20, paid common b1/v0, matched activation start4099/startA11993, fixture costs VM1310/world4371A separate. All replay, safe inventory, and actual task checks passed.

| Arm | Policy/race | Actual task | Non-task A | GC B | memory write | Final memory |
|---|---|---|---:|---:|---|---|
| RELEASED | STALE | Quota then Ok |3379|276|none|1/v0|
| RELEASED | REVISE | Quota then Ok |3415|276|step4110 b0 Ok/3A|0/v1|
| PINNED | STALE | Quota |3778|0|step4113 b1 Ok/4A|1/v1|
| PINNED | REVISE | Quota |3747|0|step4113 b1 Ok/4A|1/v1|
| STALE_CAS | REVISE no race | Quota then Ok |3415|276|step4110 b0 Ok/3A|0/v1|
| STALE_CAS | REVISE race | Quota then Ok |3412|276|external step4110 b1 Ok/3A; VM step4111 b0 Conflict/0A|1/v1|

Every successful task at4108 (first Quota4099), task fee20A separate. Every arm one full Index; RELEASED/STALE_CAS 3 Conflict before GC; PINNED 8 Conflict no GC. REVISE VM1077A successful vs STALE1016A; net observed non-task difference +36A is not a pure incremental CAS fee, because root identities/sizes differ in reference scanning. Race external3A excluded from reported non-task total, exactly one rejected VM write, task still committed and VM returns success. Serialized index peaks 3694B released,3854B pinned (unbilled, not process memory); physical storage peaks159454/159450B. No false positive recovery claim. Current b0 conflates success and cooldown reset elsewhere: follow-up should distinguish outcome state and retry scheduling, not call it autonomous learning. G0 GO/G1,G2 NO-GO; formal E001 uncalibrated/unpreregistered.
