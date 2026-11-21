# S45 组合故障交叉试验：高延迟任务漂移与 RESULT 阶段伪回执

**状态：工程测试候选，原生CI尚未给出结论。** 本文依赖实时main在 S45 [原生3/3成功](https://github.com/iFrankStudio/OCEE/actions/runs/44905343381) 后的事实，并不预设下一研究的Study编号。

## 缺口

已验证 S44 在正常时序下能从真实任务栅栏Drop恢复一次、并且面对伪造远程回复会拒绝；S45验证全程消息延迟6步可成功从C获得ACK/RESULT及B24仅一次。但**这两个集合的交集尚未证实**：S45改变了非phase1 Empty处理/Phase2等待方式，可能意外破坏旧异常路径。单独的低延迟安全测试、单独的高延迟正常测试不允许推断组合条件的正确性。

## 可证伪对照

1. **delay6 + 栅栏丢弃 + 恢复**：从启动开始将全部消息延迟固定为6。等待真实RemoteEval已SEND（从内核实际反馈取得u64 ID）；再改变真实公共任务字节及版本，确保C/B处理旧请求前版本不符，必须TaskFenceDrop旧ID/B0。正在执行的个体通过自己的 RequestOutcomeFor 认定Drop、付费受限RestartRootFor一次、重新学习并发出另一条真实RemoteEval，B24仅执行一次，最终真实RESULT成功且新ACK得到ConfirmResponseFor Ok；旧ID没有合法回复认证。验收两次RemoteEval不同u64、只有第二次付B、写出的RESULT等于真实fold32，世界快照一致。
2. **delay6 + RESULT扫描中无服务来源消息注入**：等待活动个体真正发出首个Phase2 RESULT候选，并确认计算任务B24已完成，此时向其A邮箱插入构造的C来源但无合法ServiceEvent的回复。即使 Receive 返回Ok，也必须由S43B ConfirmResponseFor 针对该候选的u64请求ID返回Conflict；不得把伪回复写成最终结果，也不得触发Root重启或再次执行B。保存快照一致性。
   
两个对照独立原生运行，各约一条真实活动P2轨迹，不重复旧的全workspace及所有Study。失败必须保留真实日志，不应随意放宽断言。科学状态以最新工程检查点为准；不将本交叉安全性证明当作开放式赛博生命或现实互联网exactly-once证明。


## 2026-11-21 原生验收：2/2 PASS（真实组合故障）

[GitHub Actions #44909113918](https://github.com/iFrankStudio/OCEE/actions/runs/44909113918) 在源码 SHA `dc7022ccdfb98120b85b613ba7c4e066fcc5c0dc` 完成，workflow / job 均 completed/success。两个独立Cargo测试步骤：

- `s45_global_six_step_delay_and_real_task_drop_relearns_exactly_once`：**1 passed / 0 failed；57.90s**。从世界构造到结束维持全局 `message_delay=6`，原真实SEND后改公开任务，任务栅栏真实丢弃旧u64请求；个体观察自己请求的 `Dropped`，一次 `RestartRootFor`，产生不同u64的新REMOTE_EVAL。两条SEND中只有新请求实际耗B24一次，旧ID不曾被确认为有效回复，新ID具有真实服务ACK因果确认；最终U32(1)、结果与真实 `fold32` 一致，快照状态哈希相等。此组合不等价于无限次任务变化。
- `s45_delayed_phase2_forged_reply_fails_closed_after_real_b`：**1 passed / 0 failed；51.78s**。全局delay6、真B已执行且耗24后，个体已经发送一个真实Phase2 RESULT候选；研究者将伪造的C来源但**无ServiceEvent背书**的消息直接注入A邮箱。真实Receive返回Ok仍不足以通过，活动个体的 `ConfirmResponseFor` 针对这一次SEND的u64消息ID返回 `Conflict`。没有写出最终 `/self/results/p2-job0/value_le`，没有Root重启/B重试，快照重放哈希一致。这是主动注入负对照，不代表防御真实互联网所有恶意代理。

两项共2/2真正运行并通过；其原生Rust构建也通过。此处没有新增S45 VM能力，只是对已交付S45的**组合故障因果边界**做原生独立验收。既有S45 [3/3 #44905343381](https://github.com/iFrankStudio/OCEE/actions/runs/44905343381) 的正常与高延迟基线不被这次2/2替代；全量workspace旧run cancelled，不可报告绿色。

实验边界：固定P2协议研究者先验、确定性C/B与受信任世界内核、特定seed和单次任务漂移、伪造消息由研究宿主直接注入；不证明现实网络 exactly-once、任意网络故障、通用自主性或开放式演化。科学阶段G0 GO、G1/G2 NO-GO；E001未完成预注册/校准。

验证完成后，`.github/workflows/s45-concurrent-faults.yml` 恢复为 `workflow_dispatch` 手动重放；保留上述真实CI URL及源码SHA供后续自动/手动研究从最新HEAD接续。
