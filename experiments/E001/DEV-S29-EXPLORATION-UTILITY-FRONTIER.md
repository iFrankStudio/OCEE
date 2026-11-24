# DEV S29 — Empirical exploration utility frontier (offline diagnostic)

Source: accepted [bounded exploration #45394634684](https://github.com/iFrankStudio/OCEE/actions/runs/45394634684) @7d51a01, test phases only. The adjacent JSON is extracted from the real CI logs (job identifiers included), with actual commits counted from the logged task tuples. For every row independently check VM_A + world_A = start_A - final_A, task_A = 20 * commits, non_task_A = VM_A + world_A - task_A. No new VM trajectory, no extrapolation to unseen tapes, no change to accepted experiments.

## Cost/utility accounting

Define analytical utility U(V) = V * successful_commits - (VM_A + world_A), with hypothetical common value V >= 0 A per independently verified commit. V is NOT an actual world reward and is NOT observable by the organism. World A grants reset each epoch; a retained balance is not yet inter-epoch accumulated fitness. This scalarization prices consumed A, not storage, elapsed time, B resources, or future memory effects. Common physical installation 6895A and identical training costs cancel in this matched comparison; standalone root installation costs are not measured by this contrast. Root byte difference 1481B should not be hidden when considering deployment.

| Tape | BASE commits / total A | PROBE commits / total A | Actual Observe count BASE / PROBE | Delta U (PROBE - BASE) |
|---|---:|---:|---:|---|
| SILENT | 1 / 2122 | 1 / 2276 | 0 / 24 | -154 |
| LATE | 1 / 2122 | 2 / 3658 | 0 / 48 | V - 1536 |
| MID_PULSE | 2 / 3832 | 1 / 2276 | 0 / 24 | 1556 - V |

The LATE case has two complete 24-Observe waits, not just one. Their world charge is 48A, but the additional overall cost is 1536A including task work, VM and memory behavior; do not mistake direct wait charges for exploration cost. MID_PULSE saves 1556A precisely while losing a correct commit. SILENT BASE strictly dominates on these two axes.

For a hypothetical mixture with pS+pL+pM=1, all nonnegative, expected delta = -154*pS + (V-1536)*pL + (1556-V)*pM. The exact decision boundary follows from the measured trajectories, not from empirical probabilities. When pL=pM, the V term cancels. At equal weights, BASE wins by 134/3 A per episode at every V; both average 4/3 commits. At V=1546, both non-silent tapes favor PROBE by 10A, but SILENT costs 154A; PROBE wins iff pS < 5/82 (approximately 6.10%). For both non-silent tapes to strictly favor PROBE, 1536 < V < 1556; this narrow interval does not demonstrate general superiority.

## Implications

- A fixed remaining-A threshold or observed-success count cannot recover value V or tape prevalence; the observation-aliasing experiment already established that unseen opportunities cannot be inferred from identical feedback.
- The current prototype lacks a task-dependent spendable resource return in these S29 local-offer episodes. It cannot yet infer ecological fitness from a researcher-assigned score. Avoid calling net A saved a learned objective.
- A future independent experiment should make successful service delivery produce a real bounded resource return through normal world accounting, separate verification from credit, specify debit source and conservation, and compare policies using observed net resource consequences. First freeze a narrow economic contract; do not add an unlimited reward oracle or silently change existing E001. The current reserve-evaluation CI is still separate and must be accepted or rejected using its frozen criteria.

Reproduction: run `python3 experiments/E001/s29_exploration_utility.py` from any working directory; the adjacent JSON holds the six audited phase records. This is an offline sensitivity analysis, not a formal E001 scientific outcome. G0 GO/G1,G2 NO-GO.
