# DEV — 单次激活的有界连续维护重试：失败记忆、VM续执行和世界tick

**状态：实现前冻结，尚未有本轮原生结果。** 主分支基线 `dab64fd7702c2fba83d0b8fb0cfc8de0c8661aef`。前次[持久反馈记忆](./DEV-S29-PAID-PERSISTENT-MAINTENANCE-BACKOFF-PROTOCOL.md)首轮原生3/3，但每一个维护episode仍是研究者通过付费Read`/sys/active-version`再CAS`/self/active`重新启动同一根。当前内核`tick_autonomous`与VM已有`Continuation.loop_counters`持久化、`Terminator::RepeatBounded`静态上界256；这次**不修改任何kernel或VM接口**，只将运行计划保持在同一个尚未Return的程序中。

## 冻结机制和科学含义

研究者仍在完成真实S29错误ByteAt二公开例修复/原子Len发布/heldout5/P2任务B20之后，**仅一次**付费安装并按真实active-version CAS激活一份研究者提供的有界维护根。候选旧程序仍是三个已合法安装内容寻址版本 `[279,276,279]B`，真实world seed129/protocol42、A每epoch12k、原16k tick任务上限，不更改能力、指令、引用索引、状态可见面或计费规则。

在旧`feedback_backoff_root`基础上将入口改为 `entry -> retry_header(RepeatBounded limit:3) -> risk_entry`，第三轮后的 exhausted 直接Return U32(0)。各轮内逻辑与之前完全一致：付费Read自身`/self/maintenance/risk-s29`，无对象或 `b"0"` 则付费Read真实`/sys/resource-status`，压力+预算满足时真Index+最多三次GC；首个真实`ReclaimIndexedVersion`返回Conflict则该VM用普通CAS None真正Write风险`1`；下一轮读到`1`则用普通CAS0真正Write风险`0`、**不建索引也不删**；随后重新允许资源条件分支。

和前次不同，所有失败/冷却分支不立即终止：它们需经一个实际`WorldCall::Observe()`（普通收费1A、真实一个world tick），再转回同一个静态`RepeatBounded`头；没有研究者重新激活、外部Tick故意重置continuation、无隐藏host计时器、无无限循环。所有写/读都来自真实活动VM，且每个已执行循环和已排队反馈保存在snapshot里。需要同时记录至少一个独立真实Observe与连续的世界step；**不声称跨过整个1024tick epoch或产生自主睡眠调度**：只有同一活动VM后续tick的有限继续执行。

## 三条预先固定的真实原生验证

1. **ONE_ACTIVATION_RELEASE**：研究者正常有偿设置世界高存储压力与足够A，实际连续`tick_autonomous`；在第一轮VM成功`IndexVersionRefs`后，研究者通过真实CAS Write添加首旧hash pin，使首次GC真实Conflict，VM自己收费Write`b"1"` v0；不重启根，VM经真实Observe前进到第2轮，VM真实Read风险1、付费Write`b"0"` v1，**第2轮不得Index**；在第2轮写0完成后研究者通过真实CAS同长度换掉pin正文，仍不重启根；第3轮VM自身在新的tick读风险0，重新收费索引并连续真GC3×Ok，释放**834B**，最后Return U32(1)。需审计仅一次研究者初始`/self/active`切换、三轮中零次该路径Write、每轮真实风险Read/Write、每个retire Code、原B20不重复、新Len真Exec、状态hash及快照。
2. **PIN_STAYS**：相同根与真实第一轮竞态，但研究者**始终不释放pin**。第二轮同样真实短期退避不扫描，第3轮会受限制再次真Index、首GC仍`Conflict`，在有界原型中`risk-s29`现在v1但失败处理只支持CAS None，额外风险写`Conflict`（不得假报记忆成功），第3轮后RepeatBounded预算耗尽安全Return U32(0)，目标三旧版均未删，B20/snapshot不变；明确说明“第三轮CAS不适应多次冲突”是实际限制，**不**后验修成支持任意版本写。
3. **SNAPSHOT_CONTINUATION**：在第一轮真实GC Conflict且VM确实已普通Write风险1后，复制完整同一world`snapshot_json`两份（包括`vm_continuation.loop_counters`、pending_feedback、已完成世界审计），各支仅运行同样普通世界tick，不再Read active-version/CAS重启；两支在相同step真实Write替换pin、恢复运行并完成相同第2轮冷却和第3轮成功删除，最终state_hash、审计轨迹、实际A/B支出完全一致；不能由宿主重建新Continuation代替恢复。

如有原生红例，记录真实报错与最小修复，不改变本协议核心条件/上限/seed。只设置一次限定scope GitHub Actions 工作流，不跑全项目老Study；最终完成后改`workflow_dispatch` only。

## 解释边界

通过仅代表**研究者编写的固定有界Program在单次活动启用后的运行时条件行为、真实错误回执持久记忆与有限自我重试**。环境写入并撤销pin仍由实验研究者控制；世界的自动`tick_autonomous`也由外部模拟驱动器逐tick调用；候选/计划/固定一轮退避/目标均是预置，不代表生物式自发目标、独立时间调度、任意长期风险学习或开放式演化。保持G0 GO，G1/G2 NO-GO，E001仍未预注册/校准。