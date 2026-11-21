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


## 2026-11-21 S45原生工程验收：3/3 通过（高延迟旧红例已修复）

[**GitHub Actions Run #44905343381**](https://github.com/iFrankStudio/OCEE/actions/runs/44905343381)，代码SHA `e66a691b0c00e8a47ce1f561dc2820c8139905d1`，workflow/job completed/success，三段独立Rust命令逐一成功：
1. `scoped_wait_budget_is_per_candidate_but_has_hard_global_cap` — 1 passed/0 failed (0.01s)。在VM内部两个不同scope分别计数、跨Yield与Continuation快照保持一致，全局3/4次数上界不会被变化scope绕过。
2. `s45_global_six_step_delay_validates_real_b_and_result_once` — 1 passed/0 failed (61.51s)。**从boot到finish全程世界消息延迟6步**，不再复用S44最终绿例中途恢复2步的实验技巧。活动P2个体最终U32(1)、B账本恰好24、真实RemoteEval仅一个SEND、ACK/RESULT分别具有真实 `ConfirmResponseFor` Ok；Phase2在多个opcode scope上消费局部预算，总额没有超过896。未触发无因重启；快照/状态哈希一致。
3. `s45_baseline_two_step_latency_keeps_correct_result` — 1 passed/0 failed (31.88s)。同新程序在原2步时正常返回U32(1)，B24仅一次，结果等于从真实远程operand计算的fold32，无无因重启。

这从已验证旧S44同一全局6步红例（[#44862804649](https://github.com/iFrankStudio/OCEE/actions/runs/44862804649)，当时B24已执行而P2最后U32(0)）取得了明确的、可以回放的工程修复，而非只修改超时断言。新VM语言机制为任意scope提供各自受限等待及全局可验证上界；**它没有让世界自动发明协议或修复未知真实网络**。

### 失败记录（保留，不计入成功）

- [S45首轮 #44905202275](https://github.com/iFrankStudio/OCEE/actions/runs/44905202275)：VM scoped单测通过，但旧S30图边诊断未涵盖新 `Terminator::RepeatScoped`，Rust E0004 编译失败。已补齐旧图枚举匹配。
- [S45第二轮 #44905261924](https://github.com/iFrankStudio/OCEE/actions/runs/44905261924)：VM单测通过且P2编译成功，但S45错误地假设 `s22_feedback` Empty分支仍直接去 `s23_receive_retry`；实际S41/S42B之前已改为 `s41_check_drop`。构图断言明确失败；修正为只接管S42B `s41_check_drop` 的非phase1出口，保留phase1因果查询和一次安全重启路径。
- 后验S45最终3/3为真正成功。此前失败的Run不予删除、没有因为新CI通过而倒填成成功。

**边界**：候选最多112种，Phase2每opcode最多6次额外Empty，每个VM继续体所有RESULT候选总共最多896次；原其他phase共享128次上限保留。若超出支持延迟的范围，可能安全失败而不是无限等待；没有证明任意丢包、现实服务请求的exactly-once、开放式演化或E001假设成立。S45定向CI只确认其3个本轮命题，非全Rust workspace；此前全仓Run #44861571121已取消，不能记绿。科学闸门仍G0 GO、G1/G2 NO-GO、E001未校准、未预注册。
