# S24 — 有界 P2 TLV 可恢复拒绝与损坏 ACK 后的 RESULT 恢复

**2026-11-19，北京时间；开发集工程记录，非正式 E001 预注册结果。G0 GO；G1/G2 NO-GO。**

## 1. 为什么需要这个实验

S21 后续真实 C 回复注入对照已证实：添加格式合法、但不在预期五字段序列中的额外内层 TLV，活动 VM 执行 `TlvReorder` 时会出现 `InvalidCode`；这只意味着没有保存伪成功，不代表个体能够恢复。

S23A 已能保存和继承可执行 P2 REMOTE_EVAL 工具，但它复用 S22 的部分回复解析路径，也可能被不受信任的 TLV 扩展打断。

S24 区分三个层次：

1. **通用 VM 受限字节安全原语**：`TlvMatchesOrder(raw,expected_tags)` 完整检查内层 TLV 字节长度、确切字段数、标签唯一性和真实物理顺序；错误/重复/额外字段**返回 Bool(false)，不以 VM InvalidCode 抛出**。计算按字节计 VM gas。预期的 tag/order 由活动个体从公开 HELLO 学出，而不是 VM 内藏私有 P2 参数。
2. **个体可控制的失败分支**：S21B 在任何 `TlvLookup/TlvReorder` 之前进行结构预检查。S22/S23 共用的真实计算响应处理也改用上述总函数，错误回应进入既有的失败逻辑，避免额外基本块和绕过 VM 静态上限。
3. **有界的任务结果恢复假说**：S24B 是**单独、研究者预设**的 S22 初始程序变体。若真实 REMOTE_EVAL 任务的 ACK 异常，个体不接纳损坏 ACK，而是支付正常世界 OBSERVE/消息和 RESULT 112候选搜索成本，尝试通过真实 C 新一轮合法 RESULT 帧取回任务。若没有结果则按有界策略失败；若得到真实 RESULT，必须再次验证五字段、校验、opcode、请求ID、结果状态和正文，不能直接相信 B 的特权账本。

## 2. 代码与真实 C 注入

- [VM 的通用 TlvMatchesOrder 与字节计费](../../crates/organism-vm/src/lib.rs)。
- [S21B 预检与可控分支](../../crates/organism-seeds/src/s21_p2_tlv.rs)。
- [S22/S23 共用响应预检、独立 S24B 恢复程序](../../crates/organism-seeds/src/s22_p2_remote.rs)。
- [S21 真实 C 的外/内层 TLV 注入回归](../../crates/organism-seeds/tests/s21_p2_outer_injection.rs)。
- [S24 真实 B 工作与损坏 C ACK 的安全拒绝/恢复对照](../../crates/organism-seeds/tests/s24_p2_recovery.rs)。
- [定向 CI](../../.github/workflows/s24-tlv-recovery.yml)。

在 world seed42、P2 protocol seed42，研究端故障注入器只在**真实 C 服务已经处理一个 B 计算请求、待交付合法二进制 ACK 时**，在其内层 TLV 后追加一个合法的空 `0x7f` TLV，并同步增大两层长度。注入器不供个体使用、不插入伪造 B 结果、也不改变研究者私有答题验证规则。

## 3. 实测及失败边界

[S24 定向 CI #44502050065](https://github.com/iFrankStudio/OCEE/actions/runs/44502050065) **成功，7 项相关测试通过/0失败**：通用 VM TLV preflight（1）、S21 真实 C 外/内层 TLV 注入（2）、原 S21 HELLO 正向与错误判据（2）、S24 真实 B ACK 注入拒绝/补救（2）。

| 相同真实 B 工作、ACK 交付前加入额外内层 TLV | 活动程序结束状态 | 逻辑世界 tick | A 总 gas | B gas |
|---|---|---:|---:|---:|
| 原 S22 安全拒绝（修改后的通用结构检查） | `U32(0)`，不保存成功结果 | 72 | 不单独作为收益对照 | 20 |
| S24B 研究者预置有界 RESULT 补救 | **`U32(1)`，取得并独立核验 FOLD32** | **1,047** | **12,912**（其中 VM 8,821） | **20** |

S24B **没有将损坏 ACK 存成合法 ACK**，也没有重复执行 B；只在后续真实 RESULT 验证通过后保存新结果。研究端以当前世界独立 FOLD32 公式核对最后四字节，证明任务确实由 B 完成而非猜测。这里增加的是研究者提供的容错选择，而非个体自主修补 VM 或创造算法。

较早的 [S24 定向 CI #44500814900](https://github.com/iFrankStudio/OCEE/actions/runs/44500814900) 首先验证 S21 内层 TLV 的恢复路径。之后 [#44501393556](https://github.com/iFrankStudio/OCEE/actions/runs/44501393556) 验证 S22 响应解析对同一故障类型的可控拒绝。**完整 workspace CI** 在报告撰写时 [#44502050056](https://github.com/iFrankStudio/OCEE/actions/runs/44502050056) 尚待最终完成；不能用定向结果替代全量验收。此前 S23A 全量基线 [#44485672595](https://github.com/iFrankStudio/OCEE/actions/runs/44485672595) **107项成功**。

## 4. 科学归因与后续风险

S24 并不是“个体自己修复了自己的算法”：通用安全原语、预检查位置、错 ACK 时转去查 RESULT 的有界策略，全部由研究者修改和赋予初始程序；S22 与 S24B 是**两个不同的初始程序**。不能把研究者的修复过程计入内源结构创新。

本轮仅对**封装长度一致且能被外层 TLV 解码器解析、但具有额外内层字段的 C 回复**给出了可控失败和补救证据；任意断帧、恶意嵌套、畸形外层、未知长度、损坏 RESULT、认证及资源耗尽仍缺更广校准，某些异常可能依旧让既有 `TlvDecode/JsonIndex` 路径报 VM fault。不能宣称所有恶意 P2 报文均安全或保证任务恢复。

下一步应在**不增加研究者特例处理**的前提下，探索把标准化失败反馈纳入个体自身可更改的控制流/工具候选，并用独立样本验证它选择、复验、保存改动和可跨 episode 转移，而非不断由研究者补手工规则。E001 正式 S/O、四组 A/B/C/D、任务收益/负迁移全面校准、预注册仍未完成，G1/G2 持续 NO-GO。

## 5. 同期完整因果回归暴露的 Empty 反馈边界

额外 TLV 的通用 `TlvMatchesOrder` 修改改变了少量 VM 消耗。随后 [S22 定向 #44501847870](https://github.com/iFrankStudio/OCEE/actions/runs/44501847870) 的**正常三种子 B 闭环**和**错误 RESULT 长度拒绝**依然通过，但“屏蔽 BadBody”阴性组在 seed42、第1028 tick 因普通 `RECEIVE=Empty` 被无条件 `JsonIndex(payload[0])` 而产生 `InvalidCode`，因此该 CI 整体为 **failure**。不能把另外两项成功说成整套通过；不能将其误归因到 P2 标签错误。

此前 S23A 已经有一套受计费的 `RepeatBounded(128)` OBSERVE→RECEIVE 修复。提交 [6abfdfa](https://github.com/iFrankStudio/OCEE/commit/6abfdfaad6634b8be2abb9244c4e9fd339ab8854) 将相同的**研究者编写**世界动作状态检查与有界重试接入 S22 的各初始变体：只有 `RECEIVE.code==Ok` 才解析 bytes、`Empty` 走有界/可持久化等待、其它状态安全失败。S23A 仍沿用其已存在的等价重试器，通过原始 S22 生成器组合，不因重复四个块突破128-block/64KiB VM 上限。

该修复正在分别由 [S22 #44503380110](https://github.com/iFrankStudio/OCEE/actions/runs/44503380110)、[S23 #44503380158](https://github.com/iFrankStudio/OCEE/actions/runs/44503380158)、[S24 #44503380118](https://github.com/iFrankStudio/OCEE/actions/runs/44503380118) 验收。**在这些后继 CI 给出最终结果前，不能称 Empty 回归已通过。** 这是先前已知的空接收时序问题在计算成本微调后被触发，不是个体自动发现的程序修复。

## S24 下沉 Empty 重试后的资源复测

[新定向 CI #44503380118](https://github.com/iFrankStudio/OCEE/actions/runs/44503380118) **success，7项通过**。真实 C 损坏 ACK：安全拒绝仍在72 tick 返回 U32(0)、B20；有界 RESULT 补救仍得到经过独立 FOLD32 验证的 U32(1)，但新成本变为 **1122 tick、A gas 18,845、其中 VM gas 14,663、B gas20**。

此前 [#44502050065](https://github.com/iFrankStudio/OCEE/actions/runs/44502050065) 的1047 tick、A 12,912、VM 8,821是**未下沉共用 RECEIVE=Empty 重试前的历史事实**，不可当作新版本成本。显式重试增加实际资源与调度成本，不能隐藏。全量回归 [#44503380220](https://github.com/iFrankStudio/OCEE/actions/runs/44503380220) 另核验。


## 6. seed99 共享 RECEIVE 预算的历史因果重放（后继工程验收）

[历史 128 预算重放与现版对照 CI #44517653790](https://github.com/iFrankStudio/OCEE/actions/runs/44517653790) **2 项测试通过**。VM Continuation.loop_counters 是按基本块 ID、跨整个 continuation 的持久计数器，不是按请求清零。旧版 S22 把远端操作码探测、B 工作 ACK 和 RESULT 查询全部汇入一个限额128的 Empty 重试块。

在研究端测试里按[历史提交 6abfdfa](https://github.com/iFrankStudio/OCEE/commit/6abfdfaad6634b8be2abb9244c4e9fd339ab8854) 的**完整共享重试图**重建旧程序，观察到 seed99 旧共享计数器**确实达到128**，程序终态 U32(0)、A gas 14197、tick1284、B gas20，RESULT 没有正确回收。原先[workspace 失败 #44503380220](https://github.com/iFrankStudio/OCEE/actions/runs/44503380220) 的 seed99 结果因此有了直接的重试预算耗尽证据，而非推测。

提交[0e8458c](https://github.com/iFrankStudio/OCEE/commit/0e8458c288802eb919d281c39dff9508f8b2a1f5) 在 S22 把该计数分为三个独立、持久、分别受限的 RepeatBounded：远端发现、真实 ACK 和 RESULT 查询；每个最大256，仍满足原 VM 静态上限、真实计费、无特权计数器清零。当前 seed99 的新程序结果正确，A gas18529、tick1117、B gas20；但本次**新三段计数都是0**：改变程序图、指令 gas 与世界调度，足以让它不再碰到 Empty。**不得把新 seed99 成功误报为新预算已真实承受超过128次 Empty**，也不能直接从新旧不同程序的 A gas 差额推断经济收益。

[新 S22 定向 CI #44516301596](https://github.com/iFrankStudio/OCEE/actions/runs/44516301596) **3 passed**：P2 seed7/42/99 的新计费 A gas为39564/18986/18529，B 各20。[新 S23 定向 #44516301537](https://github.com/iFrankStudio/OCEE/actions/runs/44516301537) **3 passed**：兼容继承的目标 A gas 从19707降至14447。[新 S24 定向 #44516301516](https://github.com/iFrankStudio/OCEE/actions/runs/44516301516) **7 passed**：损坏 ACK 后仍可以真实 RESULT 恢复，A gas18952、tick1122、B20。请勿与旧程序哈希的气体账本混用。

为了直接检验新预算的抗 Empty 能力，已在同一 P2 seed99 中加入仅改变模拟传输 message_delay=4 的独立实验，要求累计真实空接收超过原128，且每段仍符合 VM 上限、B 仍仅计费20并得到正确结果。[该扰动 CI #44518525140](https://github.com/iFrankStudio/OCEE/actions/runs/44518525140) 在撰写本段时**尚未验证**，不提前称通过。研究者实施的策略修复仍然不能计作个体内源算法发明、生命演化或 E001 确认性结果。G1/G2 NO-GO。
