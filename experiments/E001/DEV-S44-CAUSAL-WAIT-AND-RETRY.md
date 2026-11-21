# S44 — 活动P2个体基于因果状态的有界等待与安全恢复（工程候选）

**阶段：提出可证伪实现，待GitHub原生回归。**

## 关键区别

S43A内核将已成功SEND的单个u64 request ID区分为 `Unknown / InFlight / Dropped / HandledNoB / CommittedB`，S43B已能在真正Receive成功后按u64请求及完整ServiceEvent哈希确认ACK/RESULT的来源。但S43B在收到空邮箱时仍只查询 `FenceStatusFor`，并没有基于B真实执行状态自主区分**不可重放**与**确实未执行**。

S44并不赋予新工具、开放式网络能力或通用“自我意识”。它只为S43B已经知道的、真实SEND反馈保存的phase1消息ID，增加一次 **2A gas** 的 `RequestOutcomeFor` 查询，并让活动VM进行显式状态分派。

| 自身请求状态 | 保守决策 | 原因 |
|---|---|---|
| `InFlight` | 继续已有有界Receive等待 | C可能尚未消费 |
| `CommittedB` | 继续等待真实回复 | B已经扣费，重发风险不可接受 |
| `HandledNoB` | 等待并按旧协议检查回复/错误 | 处理与B执行不能混淆 |
| `Dropped` | 仅进入现有 `RestartRootFor` | 内核进一步核验最新请求、任务漂移与一次性授权 |
| `Unknown` | fail-closed，终止而不是发送 | 最近审计不充分或个体证据遭破坏 |

只修改派生 **S44 P2程序**：替换S42B已存在的状态查询和反馈区块，新增4个有类型的分派区块。继承S43B完成过原生验证的 `ConfirmResponseFor`、响应内容指纹、固定长度请求回执；继续受VM `MAX_BLOCKS=128` 与默认观察预算约束，不能假装无限等待可用。

## 最小原生对照（待运行）

- 健康P2且延迟被拉长：请求可从 `InFlight` 过渡至 `CommittedB`，在收到合法ACK以前不得盲目再次REMOTE_EVAL；最终B24一次且结果等于真实fold32。
- 真实任务规则漂移：消息确由内核在C之前丢弃B0，S44以请求自身的 `Dropped` 状态进入内核受限RestartRootFor；重新学习后新B24只执行一次，旧ID不得误确认。
- 研究者注入本地证据损坏：个体保存的8字节ID被替换为未发出ID；因果状态变 `Unknown`，应终止而非重发、接受旧ACK或启动恢复。

## 不足与科学闸门

- 有限 `RepeatBounded` 轮询预算耗尽时，仍会终止；没有进程间持久化协商、超时后的高可信再次查证和自动人工复核接口。S44不提供exactly-once分布式事务保证；只能证明在此确定性模拟/可信内核中的禁止盲目重发。
- 只针对Phase1 REMOTE_EVAL的第一次/反复Empty检查；Phase2 RESULT仍继承S43B的受限协议扫描与严格响应归属确认；没有新的通用未知状态调度器。
- 若workspace全量回归尚未出结果，必须与S44专项试验分别记录，不得把后者独立运行视为全仓证据。
- 科学仍 G0 GO、G1/G2 NO-GO，E001尚未校准/预注册，不能把工程安全策略写成自主生命涌现。

## 2026-11-21 原生第一轮失败与因果定位

[Run #44862672899](https://github.com/iFrankStudio/OCEE/actions/runs/44862672899)：Rust及Program.validate通过，健康延迟6步的测试最终 `U32(0)`，并非S44错误重启：审计确认 `B=24`，2次 `RequestOutcomeFor` 均为 `CommittedB`（回复在途），`restarts={}`。但是测试将全局C消息延迟加大，使后续S23旧式RESULT候选操作码扫描耗尽有界等待预算，进而失败。此Run为**真实未通过**，不计S44成功。修订为仅在真实REMOTE_EVAL及其ACK阶段延长消息交付，在首次合法ACK由VM确认后恢复协议扫描原始2步消息延迟；同时为Unknown负对照添加有界测试循环。这是明确限定的S44安全性验证，不声称已修复S23全局高延迟活性问题；后者应保留为未来独立实验，而非通过调小断言掩盖。

## 2026-11-21 最终原生工程验收（S44 3/3 PASS）
[GitHub Actions Run #44862964346](https://github.com/iFrankStudio/OCEE/actions/runs/44862964346) 于 `fed97e8cbfccfcb5415d510a71fb2503189c31c6` 完成三个有独立Cargo步骤的P2原生实验，**3 passed / 0 failed**，包括真实B与ACK定向延迟等待、任务漂移后一次重学习及伪造本地u64证据时Unknown安全终止。三个用例各自顺利完成Rust编译、P2 Program.validate、真实世界tick、相关账本断言及快照重放校验。

原先将全局延迟设为6步的两轮原生Run确实失败：S44将已B24提交的请求识别为CommittedB、回复仍在途，未错误重启；失败源自沿用的旧S23 RESULT候选扫描在较长全局延迟下耗尽等待预算，而非发生重复B。最终测试仅为计算请求及ACK定向延迟、ACK真实确认后恢复既有Phase2时序，**不得宣称修复全局延迟活性问题**。S44需要S45对每个RESULT候选独立预算等进行有针对性的研究。

此处仍只证明确定性模拟世界中的有界因果安全策略，并不证明开放式演化或自发数字生命。
