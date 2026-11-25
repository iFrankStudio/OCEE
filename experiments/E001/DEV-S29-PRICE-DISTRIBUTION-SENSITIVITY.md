# DEV S29 — environment-distribution and installation sensitivity
Source: native evidence 45589101624 / f201f27, 18 trajectories archived in evidence/s29-paid-price-f201f27.json. This is offline reweighting of those deterministic trajectories, not additional native runs. The pending BOUND_QUOTE test is not used. Let p denote a hypothetical frequency of the high third price 400 (low is 1), with all other fixture features fixed. No observed population distribution, no inference of p by the organism, no generalization to other task tapes.

## Exact conditional terminal balances

STOP=231. CONTINUE=60+399p. The best of these two switches at p=171/399=3/7.
QUOTE at fees 8,32,96 respectively: 139+228p, 115+228p, 51-44p. The last line reflects observed high-price failure and must not be extrapolated as fee-only overhead. For fees8/32, quote earns the same credits as corresponding fixed action but pays fee+84 extra. Best-fixed relative payoff is min(228p-84-fee,87-fee-171p) in that successful two-arm regime.

| Fee A | Strictly positive p interval | Maximum gain A per episode | Equal mixture gain A |
|---:|---|---:|---:|
|8|23/57 < p < 79/171 (approx .4035–.4620)|40/7 at p=3/7|-6.5|
|32|none|-128/7 at p=3/7|-30.5|
|96|none|-180 at p=0|-230.5|

Thus fee8 is not uniformly dominated by the two current fixed controls, but its ideal positive interval is narrow and its maximum operating advantage is only 5.714 A per episode. Fee32 and fee96 are dominated by the best fixed choice for every p in [0,1]. At fees8/32 the fee threshold for any strictly positive operating gain under these same trajectories is fee<96/7 (13.714 A); this is algebraic sensitivity, not a measured continuous fee sweep. In particular do not extrapolate through the liquidity cliff at fee96.

## Installation sensitivity

Actual installation STOP/CONTINUE/QUOTE 533/534/702 A, in a separate epoch. At the optimal p, even against the smaller additional install cost 168A, at least 30 identical independent-equivalent episodes are required for positive cumulative advantage (168/(40/7)=29.4). Against STOP at this tie the extra cost is 169A, also 30 episodes. This calculation assumes one installation, repeated independent-equivalent paid production opportunities, no replacement/maintenance/other storage costs, comparable grants, and that balances can be aggregated across episodes. Current epoch reset does not implement this lifetime economics; no claim of actual amortization or persistent learning.

## Stronger historical comparator (cross-protocol)

The unchanged legacy hidden-price regression in the same run measured simpler RAW=224 and ENCODED=67+399p, each installed at 484A. Those two switch at p=157/399. Against the better historical root, fee8 quote is positive only if 85/228<p<72/171, max 33/7 A at p=157/399, and equal mixture gain -13.5 A. Additional installation 218A would require at least 47 independent-equivalent episodes at that maximum. These are cross-protocol counterfactuals, not direct paired controls in the priced world. A subsequent deployment comparison should run the simpler roots in the same opt-in world before claiming superiority. Legacy and opt-in worlds differ in quote availability even when no quote is issued.

## Decision

Do not select QUOTE as the default or learn a probability distribution from this deliberately selected fixture. Finish BOUND_QUOTE native evidence first. If it reduces processing overhead, recompute actual conditional payoffs, positive p region and separate installation sensitivity, then test across held-out tapes and liquidity before any deployment claim. If no gain or no acceptable compatibility, preserve the negative result instead of tuning to this fixture. No change to experiment code, grants, thresholds or CI in this audit. G0 GO / G1,G2 NO-GO.
