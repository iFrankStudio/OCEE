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
