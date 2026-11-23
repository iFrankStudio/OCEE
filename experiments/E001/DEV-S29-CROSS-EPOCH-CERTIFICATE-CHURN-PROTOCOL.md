# DEV — S29 真实写入时序、认证索引失效与跨 epoch 维护机会成本（预冻结）

**状态：实验协议预冻结；原生结果待运行。** 基线 `c17f65fd84f43d970b2e0bd6f16f2ef11bab2c02`（2026-11-23），此前同根有界 GC 风险记忆 4/4 [#45145971582](https://github.com/iFrankStudio/OCEE/actions/runs/45145971582)。本次先提交协议，再提交代码/CI；不得先看结果再改阈值或成功判据。

## 问题与对抗假设

上一轮证明真实重复 Conflict 能以有偿版本 CAS 保存风险记忆，但未证明索引保留概率与资源效率。当前索引对**任何相关世界对象写入**保守失效，即使实际新写入只有 1 字节、**不包含任何旧程序 hash**，也可能导致付费 IndexVersionRefs 已消耗大量 A、下一步 ReclaimIndexedVersion 得到 Conflict；风险位只表示“GC失败”，并不区分「有危险引用」和「证书被无关写入撤销」。检验**写入时序**而非新机制带来的真实浪费，特别是之前没有测量过的跨 epoch VM continuation。

## 固定实验环境与原生流程

- **世界/P2**：原有 S29 世界 `seed129`、协议 `seed42`、A 12,000/epoch；先由原真实 S29 修复根执行完整 B20 任务，再由研究者正常付费安装原有三个可回收程序，总大小固定 **834B**，通过读实际 active-version + 真 CAS **只激活一次**与上一组相同的 `versioned_self_retry_root`；空间容量仍为当前 used+2048。
- **活动个体**：同一内容寻址维护根（非改写出不同 arm 的根），真实 `Read("/sys/resource-status")`、真实 `Read("/sys/self-object/maintenance/risk-s29")`、严格 `RepeatBounded(limit=3)`、同三候选/25%压力/2200 A门槛；真实 IndexVersionRefs、ReclaimIndexedVersion、普通 CAS Write 与 Observe，真实审计，**无任何 VM/内核功能新增**。
- **外部受控写入**：只使用 `Action::Write`，actor=原自身，用明确独立键 `/self/noise/s29-churn` 写入单字节 `x`，不包含64字节程序hash、不修改registry/pin/active。相同一次合法付费写，唯一变化为发生在索引前还是索引后。该写是**研究者提供的环境干扰**，不是个体自发行为。
- **epoch 测试**：只在真实第一次 GC Conflict 且 VM 有偿写入 risk1 v0 之后，将原持续 VM 挂起、用合法 `World::tick(None)` 让世界时钟跨越一次 epoch 边界及完成下一 epoch 的资源grant；不改动 VM continuation，不重新激活 active，不免费补充储存，严格用单世界 snapshot 完整复制两支后各自恢复同一行动，验证全部 audit/snapshot 相等。
- **账本**：固定从维护根刚激活后的同一账本基点（构建世界是共享常量）；对比 VM A、Action A 增量、Index 的实际次数及每次收费、GC Code/收费、风险内存版本、A可用额度、storage_used 差异、B是否仍为20、step/epoch。全局历史A加总不得误作本研究净收益；A gas 与回收B在不同维度，**禁止直接相减**。

## 预注册 4 臂，不追逐新随机 seed

1. **QUIET**：没有干扰写入，活动根一次付费认证Index，GC三次Ok、回收834B、风险对象保持不存在，返回U32(1)。
2. **BEFORE_INDEX**：维护 VM 第一次动作之前，研究者先付费写1B噪声；预期新索引仍有效，仅Index一次、GC三次Ok、回收834B、无risk对象，返回U32(1)。这条和下一条的环境负担与语义引用内容相同，用于隔离**时序而非是否写过**。
3. **AFTER_INDEX**：VM第一次实际成功 Index 的审计记录出现后才付费写相同1B噪声。索引被废止、首次GC真实Conflict且不删除；个体付费写risk1 v0，第二轮读取风险并冷却CAS0 v1不重建索引，第三轮重新索引并3次GC Ok回收834B，最终U32(1)。应比 BEFORE_INDEX **多一次真实付费大扫描**，显示无害写入造成的可避免机会成本。
4. **EPOCH_GAP**：与 AFTER_INDEX 使用同样的真实索引后写入和首次 Conflict；VM写risk1 v0后冻结当前世界，在没有任何VM动作的 `tick(None)` 中跨至少一次 epoch 界，内核自己按已有逻辑grant新epoch资源；继续原VM第二轮冷却CAS0 v1、第三轮重新索引成功回收834B、U32(1)。在 epoch 边界以真实snapshot复制两支，两支终局完整snapshot和audit逐字节相同；B保持20。它证明**研究者驱动的时钟跨度内状态连续性**，不证明自主醒来。

## 不变量、失败解释与拒绝条件

- 所有臂未授权 root 切换次数=0；旧程序被回收前不改变工具注册、已存在新Len工具或P2结果；不重新执行原 B20 服务。每次世界调用要在审计中记录真实 Code 和 gas。
- AFTER_INDEX 与 EPOCH_GAP 必须有真实 `IndexVersionRefs` 先于无害写入先于 `ReclaimIndexedVersion Conflict`，不能用直接编辑内部 `version_ref_index` 制造失败。若干扰写入不使索引失效，结果即为原假设被证伪，**不得改成恶意 pin 以凑成功**。
- BEFORE_INDEX/AFTER_INDEX 有相同1B噪声及相同三个候选，无 hash 原始文本引用，研究者不切换根；如不同日志A账本，必须分开VM A、世界动作A、环境干扰A和epoch grant，不可只报告一个数。
- 若跨 epoch 的背景环境发布新任务而产生新的真实 B 计费、GC活引用或资源干扰，应报告为边界失败，不调整seed/epoch_length来遮掩；不得将 `tick(None)` 误报为个体自主决定执行。
- 如果本次CI编译红，记录真实红绿链，仅修正类型/受信夹具错误，不放松固定研究条件。CI以一次路径限定的4-arm Rust Actions真实跑完为准，验收后恢复 `workflow_dispatch` only。

## 科学结论的上限

有害与无害的索引失效都由**内核保守失效**导致；个体只看到实际 Conflict 和风险记忆，不知道究竟是哪类干扰。本轮最多确认「证书有效期、付费扫描频率和跨epoch记忆连续性」的因果可测；不证明长期演化优势、现实环境写入频率、动态候选发现、自发维护目标或跨A/B单位净效用。此前所有S29开发数据仍非正式E001预注册统计实验；**G0 GO / G1/G2 NO-GO；正式E001未预注册/校准**。
