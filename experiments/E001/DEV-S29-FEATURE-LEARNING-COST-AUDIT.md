# DEV S29 — Cross-study actual feature learning cost audit

Sources: #45534972346 job138939230995 and #45535283019 job138940256351, 36+36 native log trajectories. JSON contains parsed actual settlement counts (tuple counts), credits, fees, final A, payer A, Gas, paid root size/install, task tape, grant and arm. Independent extraction checked grant+credit-fees=final, 1400-credit=payer on all72. This is descriptive matched-fixture comparison, not randomized causal isolation of every learning instruction. All roots run same 15tick tape/grid, separate paid installations; initial grant is after install, so installation delta is separately reported, not subtracted from final balance as if same epoch.

| arm | total tasks | credit | fees | final A | Gas | summed install |
|---|---:|---:|---:|---:|---:|---:|
| RAW |47|5818|22321|6537|35|8712|
| ENCODED |47|5818|22321|6537|35|8712|
| FIXED_RAW with fitting |46|5618|22913|5745|42|12132|
| SELECT |47|5818|23073|5785|42|12132|

SELECT vs matched FIXED_RAW: one extra settlement (dense-to-sparse1152), +200 credit, +160 fee, +40 balance; other17 pairs same. SELECT vs simpler RAW: equal total task/credit, +752 runtime fees, -752 final balance, +7 Gas, +190A/installation, +3045B root, 17/18 final balances lower. Dense-to-sparse1152 +1 task but -15A final, sparse-to-dense1152 -1 task/-127A/7 extra Gas. Sparse-to-dense896 has +173A despite same seven Gas/unfinished, not successful resource management. Aggregate grid is not a measured population distribution, repeated installations are independent experimental worlds and not a lifetime fitness metric. No evidence to deploy selection as default. Further model complexity deferred; investigate missing actual revenue information, which no cost predictor alone can resolve.

Online feature acceptance #45535283019 @17818e1: all36 actual accounting, receipt, continuation histories/flag, matched first intervals and replay passed. Roots10648B/install674A. Dense interval221 then526–527 chose encoded; sparse221 then394–395 chose raw. DENSE1152 matched fixed2/final169A, select3/final209A, no Gas. Mechanism accepted, deployability not established. Full workspace remains cancelled.
