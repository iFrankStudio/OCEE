# OCEE maintenance feedback: matched lifetime cost/benefit protocol (design only)

Status: **preregistered design; NOT implemented or executed**. Based on main 93584b5ce82db3ee94589ecfbf3ed867714d5e77 and the four-arm native success #45206316515. This is a separate *engineering utility* test, **not formal E001 preregistration**.

## Question and falsifiable hypothesis

Does one-round negative-outcome memory improve *cumulative ability to complete useful tasks under finite A and storage*, compared with the same paid resource-gated maintenance without cooldown? The preceding DRY_REPEAT's 3024A first attempt versus 85A skipped episode compares different states, not paired policies. An improvement in GC bytes or one episode's A alone does not establish lifetime benefit.

Hypothesis H1: under sustained true all-pinned conditions, feedback cooldown reduces cumulative A and does not reduce useful-task completion relative to always-retry. H2 (counterexample): if a reclaimable candidate appears during the skipped round and storage blocks a task before the next probe, cooldown can lose task completion despite lower A. Both outcomes must be reported; a null or negative result is valid.

## Fixed design before coding

Use an identical genuine P2-completed world snapshot, owned versions, initial A/B/storage and installed task payload for each paired branch. Keep true-world pin writes, releases, and environment changes on a **frozen exogenous event tape** (same tick/epoch and same actor actions), not conditional on policy outcomes. Never directly mutate the index, A balances or hidden eligibility. Use actual charged Read, Write/CAS, IndexVersionRefs, ReclaimIndexedVersion, Observe, snapshots and audit. Distinguish exogenous research setup costs from agent VM and world-action costs. Preserve B20 historical charge and prohibit re-executing B.

Compare (A) resource-gated always-retry with no negative memory, (B) one-round cooldown feedback, (C) no-maintenance control. Give all three the same external activation opportunities and finite task budget; do not describe these externally supplied opportunities as endogenous wakeups. A/B must use the same catalog ordering, index mechanism, per-attempt bounded search, threshold and program-version inventory. Record exact policy hashes, snapshot SHA, event tape hash, seed and code commit before running. Include the **memory bytes and paid CAS costs** for B; include policy program bytes, index serialized peak bytes and quota accounting status in all branches. Do not pretend the current uncharged index serialization consumes organism quota.

Minimum preregistered tapes: (1) all-pinned throughout, repeated zero-yield; (2) one real 64B pin released just before a B cooldown round, with a storage-blocked useful task before its next probe; (3) release immediately after cooldown; (4) mixed reference churn with at least two owners of a hash and an in-flight transport reference; (5) no reclaimable versions. For each tape run deterministic replay from the same initial snapshot and independent replay restoration. Keep tape order fixed; do not tune after observing outcomes. Use at least 3 deterministic seeds for preliminary directional evidence, with each pair sharing the seed; this is not statistical proof or E001.

## Metrics and safety gates

Primary: completed externally specified useful tasks before the same finite horizon and resource budget, plus count of resource-induced task failures. Secondary, separately: A spent by VM, A spent by maintenance world actions, A spent by exogenous actions, A remaining; B spent (not converted to A); peak and final organism storage bytes; index serialized peak bytes **reported separately if uncharged**; GC attempts/conflicts/successes and bytes actually reclaimed; catalog/index counts; missed reclaim opportunities and their task impact; epoch/tick counts. Do not subtract gas from bytes or combine uncharged index bytes with quota.

Safety invariants: no live version deletion, no stale CAS overwrite, no hidden free observation, no Unknown=>not executed, no duplicate B charge, no mismatch in externally frozen tape, and exact snapshot/audit replay. On any invariant violation, mark the arm failed and retain logs, rather than cherry-pick surviving pairs. For every tape compare policies at the *same external schedule*, including explicit external activation. Report per-pair deltas and the full adverse case; avoid an average that hides a failed useful task.

## Execution plan and decision

First add the smallest native Rust paired fixture to the existing S29 organism-seeds test target, preferably one tape at a time. Run the precise test locally if Cargo is available; otherwise use one manually scoped workflow, never the historical workspace matrix. Inspect raw job logs and preserve failed attempts. Do not claim passing until the exact head SHA's native jobs completed/success. Only after evidence, update 07-ENGINEERING-CHECKPOINT.md with real numbers and limits.

This experiment evaluates a **researcher-authored finite feedback policy**, not spontaneous objective formation, endogenous activation, open-ended adaptation, or cyber life. G0/G1/G2 and E001 status must be re-read from the live checkpoint before interpretation.
