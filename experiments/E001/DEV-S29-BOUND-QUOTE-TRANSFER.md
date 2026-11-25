# DEV S29 — bound quote native audit and transfer gate

Native source e4f6463b0ef081949b46ed702ca0c988e05c29f0, run [45612707970](https://github.com/iFrankStudio/OCEE/actions/runs/45612707970). Evidence branch research-evidence/s29-paid-price-e4f6463b0ef081949b46ed702ca0c988e05c29f0-1. All four tests exit0; 24 ablation trajectories independently verify 1152+credit-fees=final_A, 1400-credit=payer_A and replay=true. Original three controls unchanged.

| fee | STOP low/high | CONTINUE low/high | QUOTE low/high | BOUND_QUOTE low/high | BOUND minus best fixed equal-weight mean |
|---:|---:|---:|---:|---:|---:|
|8|231/231|60/459|139/367|156/384|10.5|
|32|231/231|60/459|115/343|132/360|-13.5|
|96|231/231|60/459|51/7|68/7|-222|

BOUND reduces fee-exclusive running overhead from84 to67A for successful 8/32 arms, installation702->654A, root11103->10333B. At96 high still 6 Gas, no third attempt, no return. At fee8, BOUND=156+228p, positive vs these fixed controls only for 25/76<p<32/57; peak159/7 A at p=3/7. At equal mixture +10.5A, extra install120A vs CONTINUE requires at least12 independent-equivalent episodes for strict positive cumulative gain; actual epoch resets and lifecycle economics remain unvalidated. Against the stronger legacy RAW/ENCODED comparator the cross-protocol equal-weight gain is only 3.5A (270 vs266.5), installation gap170A; need a direct comparison in the same opt-in world. Do not promote default.

## Frozen transfer experiment

Retain the exact previously accepted roots RAW, ENCODED and BOUND_QUOTE. No threshold retuning (159A), no kernel or VM changes. Direct same-world opt-in controls, three tapes DENSE_TO_SPARSE (selected prior fixture), SPARSE_TO_DENSE (previously studied cost tape, new price comparison), MIXED_MID (new deterministic tape, length96 repeated 11 then 192 alternating 7/255). First task unchanged, second price200, third price1/400. Grants1024/1152/1408, fees8/32, seed129, epoch16, endowment1400, 15 ticks. 108 native trajectories. No expected minimum settlement count or guaranteed high-price completion; record failures without adaptation. Require actual quote scope c/2, paid/no credit, low-price no third attempt by BOUND, high-price third attempt only after successful quote, actual ledger, receipt correctness and exact replay. Raw/encoded never query, may stop or continue; actual production decisions not forced. Report per-cell paired equal-weight final A delta against the better simpler root, task count, Gas, installation and root sizes. Mixture is hypothetical, tapes selected by researcher, not sampled population. No lifetime claims or deployment selection solely from selected positive cell. Pending native. G0 GO/G1,G2 NO-GO.
