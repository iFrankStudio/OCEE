#!/usr/bin/env python3
"""S26 exploratory C/D paired episode CSV audit; NOT E001 confirmation."""
import csv, json, sys
from collections import defaultdict

FIELDS = {"pair_id","arm","source_family","target_family","success","a_gas","b_gas","bytes_sent","ticks","causal_tool_used","seed_hash"}
COSTS = ("a_gas","b_gas","bytes_sent","ticks")

def audit(rows):
    paired = defaultdict(dict)
    for row in rows:
        if not FIELDS.issubset(row):
            raise ValueError("missing required CSV fields")
        pid, arm = row["pair_id"], row["arm"]
        if not pid or arm not in ("C","D") or arm in paired[pid]:
            raise ValueError("invalid or duplicate pair/arm")
        if row["source_family"] not in ("P0","P1","P2") or row["target_family"] not in ("P0","P1","P2") or row["source_family"] == row["target_family"]:
            raise ValueError("expected distinct supported protocol families")
        if any(row[f] not in ("0","1") for f in ("success","causal_tool_used")):
            raise ValueError("binary flag is not 0 or 1")
        parsed = dict(row)
        for k in COSTS:
            parsed[k] = int(row[k])
            if parsed[k] < 0:
                raise ValueError("negative cost")
        paired[pid][arm] = parsed
    if not paired:
        raise ValueError("empty sample")
    both = []
    diffs = []
    by_arm = {"C":0,"D":0}
    causal = 0
    details = []
    for pid, pair in sorted(paired.items()):
        if set(pair) != {"C","D"}:
            raise ValueError("unpaired episode")
        c, d = pair["C"], pair["D"]
        if any(c[f] != d[f] for f in ("source_family","target_family","seed_hash")):
            raise ValueError("mismatched paired world")
        sc, sd = int(c["success"]), int(d["success"])
        by_arm["C"] += sc
        by_arm["D"] += sd
        diffs.append(sd-sc)
        causal += sd * int(d["causal_tool_used"])
        if sc and sd:
            both.append((c,d))
        details.append({"pair_id":pid,"direction":c["source_family"]+"->"+c["target_family"],"C_success":sc,"D_success":sd,"D_claimed_tool_use":int(d["causal_tool_used"])})
    return {
        "status":"DEVELOPMENT_ONLY_NOT_CONFIRMATORY",
        "paired_seeds":len(paired),
        "success_counts":by_arm,
        "paired_success_difference_D_minus_C":sum(diffs)/len(diffs),
        "D_success_with_claimed_tool_use":causal,
        "both_success_count":len(both),
        "conditional_D_minus_C_cost_if_both_success":{k:(sum(d[k]-c[k] for c,d in both)/len(both) if both else None) for k in COSTS},
        "pairs":details,
        "limitations":["conditional cost comparison is selection-biased", "tool provenance claims require independent world audit", "synthetic CSV is not experimental evidence"]
    }

if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("usage: python3 s26_paired_audit.py pairs.csv")
    with open(sys.argv[1],newline="",encoding="utf-8") as f:
        print(json.dumps(audit(csv.DictReader(f)),ensure_ascii=False,indent=2))
