# S45 — 候选操作码作用域 + 全局双重有界等待实验

**状态：工程候选，等待原生 CI。不得提前声称通过。**

## 证据依据与要证伪的假说

最新主线 [VM作用域诊断](./VM-BOUNDED-LOOP-COUNTER-SCOPE.md) 证明，S23 `s23_receive_retry` 的 `RepeatBounded(limit=128)` 由整个VM继续体按block ID累计，不因RESULT候选opcode切换而重置。S44在全局消息延迟6步的真实红例中，B24已执行、CommittedB可见且无错误重试，但Phase2 RESULT扫描失败。必须将共享的候选搜索预算与每条请求的等待预算解耦，并保留安全上限。

通用VM新增 `RepeatScoped{limit,aggregate_limit,scope,...}`，在既有、随快照保存的loop_counters中记区块总次数及`block_id:U32-scope`局部次数。类型检查要求U32 scope、单scope上限<=256、全局<=4096且非零；有限总额防止主体每轮选择新scope而逃逸终止校验。VM单测在两个scope切换、模拟WorldCall Yield和快照重载中分别验证局部2次与总4次，以及总3次限制不被新scope突破。

S45派生S44 P2专家，仅在 **Phase2 RESULT** 的Empty RECEIVE与S44安全状态等待路径上引入`s45_wait_dispatch`，根据真实phase寄存器选择原S23等待或新的`s45_result_retry`。结果操作码scope取VM实时候选opcode寄存器601，单candidate最多6次Empty，所有candidate合计不超过896次。原S23在其他phase的128总预算不改；S43B真实u64 SEND->RECEIVE源校验与消息全文hash、S44 RequestOutcomeFor及Unknown fail-closed都不改；不改变宿主/服务权力，也不新增自动重发。

## 必须通过的定向原生测试

1. VM层：两个scope在Yield+快照之间正确单独计数，总额度即使变scope也不突破。
2. 活动个体：从头到尾 **全局 message_delay=6**，必须完整产出真实RESULT的 `Value::U32(1)`、真实C/B 24 gas仅一次、只有一个真正REMOTE_EVAL SEND；实际ACK和RESULT都用内核 `ConfirmResponseFor` 成功验证，多个不同candidate作用域计数保留且总<=896。
3. 基准对照：同程序全局delay=2，实际B和未见输入结果仍正确，且无无因重启。

若高延迟仍失败，记录真实VM分支、候选编号、循环counter、B服务事件与最后失败原因，而不是放宽断言。原 S44 红例不更改；仅针对已验证的确定性世界而非现实互联网保证。G0/G1/G2及E001状态以仓库最新检查点为准。
