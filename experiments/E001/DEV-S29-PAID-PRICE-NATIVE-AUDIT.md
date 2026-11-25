# S29 paid price decision native audit

Source f201f273a04661a02eda93b157eb6931249db476; [run 45589101624](https://github.com/iFrankStudio/OCEE/actions/runs/45589101624); evidence branch research-evidence/s29-paid-price-f201f273a04661a02eda93b157eb6931249db476-1. Three tests exit 0; native_job_result=success. 18 policy trajectories independently extracted and checked against 1152+credit-fees=final_A and 1400-credit=payer_A. All exact replay. Contract fee8/32/96 final A256/184/88, payer50; last Ok/Ok/Gas. Hidden-price baseline exactly reproduced.

| Fee | STOP low/high | CONTINUE low/high | QUOTE low/high | QUOTE minus best fixed mean |
|---:|---:|---:|---:|---:|
|8|231/231|60/459|139/367|-6.5|
|32|231/231|60/459|115/343|-30.5|
|96|231/231|60/459|51/7|-230.5|

At fee8/32, QUOTE chooses correctly (2 low,3 high) with no Gas; incremental total cost over the matching fixed action is fee+84 A, equal in both arms. At fee96 high, quote succeeds but production cannot complete: no third attempt, 6 Gas, return=None. STOP/CONTINUE/QUOTE installs 533/534/702 A and roots 8391/8413/11103 B. Additional quote installation cost 168–169 A, charged in prior epoch and not subtracted from terminal balances. No deployment win; fixed controls have different overhead from earlier RAW/ENCODED. Earlier 57c2c81 run 45563810505 failed contract compile; subsequent source fixes and run are distinct. This is selected fixture/equal weight, not population fitness.

## Next frozen ablation

Add BOUND_QUOTE (mode3) to existing STOP/CONTINUE/QUOTE, same fee/fixture/horizon, 24 native trajectories. Existing kernel synchronous READ checks actual actor, task identity, current version, unpaid state and fee before returning. VM retains exact task-derived request, code guard and price parse/159 threshold, but removes redundant response identity and version parsing; this is valid only for the current trusted synchronous kernel contract, not asynchronous or external untrusted data. No world/VM permission or price format changes, original three roots/tests preserved. Verify actual query/path/returned payload, correct receipts, ledger, replay. Report costs and outcomes; expect reduced overhead at fee8/32 but no required profitability or high-fee success. Compare to best fixed and original quote, installation separately. No learned valuation. G0 GO/G1,G2 NO-GO.
