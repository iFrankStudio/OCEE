# S18 — 活动 VM P1 发现、远端执行与 RESULT 识别消融（开发集）

日期：2026-11-18。**仅开发实验；G1/G2 NO-GO；正式 E001 未启动。**

## 已有可验证基线
- [b5517a9](https://github.com/iFrankStudio/OCEE/commit/b5517a9012c8c0273d2cbc6b28616996aa6c3402) 的活动 VM 在已知 P1 字节填充、校验与报文骨架下，从公共 HELLO 推导实例校验键，通过有界诊断扫描寻找 REMOTE_EVAL opcode；随后通过普通 SEND/RECEIVE 提交三 u32 FOLD32 工作项。其 [CI #44310102801](https://github.com/iFrankStudio/OCEE/actions/runs/44310102801) 已完成成功。
- [6f428e6](https://github.com/iFrankStudio/OCEE/commit/6f428e6d5b5754f62667d021a73cd435743ede33) 又在同一 VM 内添加 RESULT opcode 扫描、响应结构检查、request_id 核对及结果持久化；对应 CI #44311202557 后来被后续同分支测试提交取消；完整证据使用最终 #44311562315。

## 新增机制消融（本轮）
- [67fcf96](https://github.com/iFrankStudio/OCEE/commit/67fcf966b988bf9f83a6d796b0de5416ddf53c26) 添加仅将 RESULT 响应长度期望从 14 改为 15 的受限程序变体，保持先前 opcode 探索、实际 B 执行和其他状态检查不变。
- [9aeb3c4](https://github.com/iFrankStudio/OCEE/commit/9aeb3c408dd6ad98a17e0c3a2873e5fd273ba4f3) 添加单个聚焦测试：seed 42 下应已保存 REMOTE_EVAL 探索记录及 ACK，B gas 应为 20，但不保存最终结果，VM 返回失败。这隔离“完成远端计算”与“正确识别并持久化结果”，防止只凭 B gas 误判闭环成功。
- [本轮 CI #44311562315](https://github.com/iFrankStudio/OCEE/actions/runs/44313014356) 最终已成功：**81 项 Rust workspace 测试通过、0 失败**。包含 3 个开发 seed 的活动 VM RESULT 回收与独立结果核验、无诊断负对照、RESULT 帧形状错误但仍耗 B 的反事实；B 每次实际计算计费 20 gas。

## 能力来源与限制
P1 分隔符/转义、内层布局、XOR 规则、112 次候选范围、BadBody 选择、固定 FOLD32 请求形状、ACK/RESULT 字段和响应匹配策略均为研究者初始程序先验。隐藏 opcode 与工作项数值不预置；实际搜索由活动 VM 执行并计费。该测试仅验证预置识别规则的必要性，**不证明陌生协议语法自主发明、工具创新、跨 episode 学习或开放式演化**。保留诊断反馈失效、双来源共同造假、错误继承等既有负面结果。

## 进一步成本与安全审计

新增 `ByteStuffEncode/Decode` 及 `PackU16Be` 是通用 VM 算子，按执行操作与字节长度收费。S18 主程序完整计入活动 VM 的指令 gas、A 的外部动作与 B 的计算费用，与 S17 **外部 Rust 算法不计 VM CPU** 的资源账本不可直接当作同口径 A gas。新增的 CI 专项将用 `--nocapture` 输出实际 VM gas、动作费用与 tick。**这仍是 P1 专用策略的成功，不代表能生成新协议语法、可迁移工具或已过 G1/G2。**

## S18 完整生命周期资源结果（最终 CI #44313014356）

[最新 CI #44313014356](https://github.com/iFrankStudio/OCEE/actions/runs/44313014356) 通过 **81 项 Rust workspace 测试 / 0 failed**，含 ByteStuffEncode/Decode 的保留字节转义、非法尾转义/非法 escape pair 拒绝及 PackU16Be 端序测试；另外重新单独执行 S18 三项关键测试并启用 `--nocapture` 输出真实计量。

| P1 开发 seed | SEND 动作次数（含探测与任务） | 世界 tick | VM gas | A 生命周期总 gas | B gas |
|---|---:|---:|---:|---:|---:|
| 7 | 177 | 2,270 | 29,149 | 31,446 | 20 |
| 42 | 23 | 124 | 3,779 | 4,998 | 20 |
| 99 | 125 | 1,331 | 20,385 | 22,318 | 20 |

**口径说明**：A 总 gas = 世界审计链中的实际 VM 执行费 + A 的世界动作费，覆盖初始程序安装、发现/数据记录、发送/接收/观察与 RESULT 取回；B gas 是 C→B 单次三元素计算费（20），**不同节点的资源账户不应机械加总**。默认 A 每 epoch 12,000 gas，seed 7/99 在多 epoch 中通过，seed 42 不需等新 epoch；世界 tick 包含等待消息和资源授予的逻辑时间，不等于实际 CPU 运行耗时。SEND 次数含合法计算请求及所有扫描帧，不能直接当成候选数。

**与 S17 参考求解器不可直接比较**：[S17](./DEV-S17-THREE-FAMILY.md) 的 P1 seed 7/42 仅将参考 Rust 宿主的网络世界动作记为 A gas 707/87，**没计入外部搜索/语法求解算法的 CPU/VM gas**；S18 中相应 A 总 gas 为 31,446/4,998，涵盖真正的活动 VM 决策。其显著差异首先来自资源计量口径及已预置策略的实现成本，不能说新算法变得“真实更差”或据此比较两个方案的绝对算力效率。

**审计的开发初始程序哈希**：`sha256:78a0498a036c7f713269336978ebc973bc028f3ee35a90d04ba8e3c022455e4a`（CI manifest 实际输出）。该哈希只绑定 S18 P1 专用开发程序，并非 E001 正式冻结 S 基线或可进化的新版本。

下一最关键实验：让活动 VM 把已探索的语义抽象成可再次 `EXEC` 的通用工具，测试跨 episode 的能力继承/负迁移与计费；进一步引入未在初始 VM 中完整硬编码的 P2 结构搜索。G1/G2 仍 NO-GO。
