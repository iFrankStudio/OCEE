"""Offline exact utility sensitivity; no world mutation or training."""
import json
from fractions import Fraction
from pathlib import Path

rows = json.loads(Path(__file__).with_name("s29_exploration_utility.json").read_text())
by_arm = {}
for r in rows:
    total = r["vm_A"] + r["world_A"]
    assert total == r["start_A"] - r["final_A"]
    assert r["task_A"] == 20 * r["commits"]
    assert r["non_task_A"] == total - r["task_A"]
    group = by_arm.setdefault(r["arm"], {})
    assert r["policy"] not in group
    group[r["policy"]] = (r["commits"], total)

assert set(by_arm) == {"SILENT", "LATE", "MID_PULSE"}
delta = {}
for arm, group in sorted(by_arm.items()):
    assert set(group) == {0, 1}
    a, b = group[0], group[1]
    delta[arm] = (b[0] - a[0], b[1] - a[1])
assert delta == {"SILENT": (0, 154), "LATE": (1, 1536), "MID_PULSE": (-1, -1556)}
for v in (0, 1000, 1536, 1546, 1556, 2000):
    values = {arm: v*q-c for arm, (q,c) in delta.items()}
    print(f"V={v}: {values}; equal-weight delta={Fraction(sum(values.values()),3)}")
assert Fraction(10, 154+10) == Fraction(5,82)
print("V=1546: PROBE wins iff hypothetical silent share < 5/82")
