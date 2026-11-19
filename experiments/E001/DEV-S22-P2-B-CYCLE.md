# S22 — 从公开 P2 TLV 样例到隐藏操作码、真实 B 计算与结果回收

**日期：2026-11-19（北京时间）**。本记录为**开发集工程证据**，不是预注册 E001 的确认性结果；G0 GO，G1/G2 NO-GO。

## 目标与对 S21 的真实增量

S21A 在一个活动 VM 中从公开 HELLO request/response 推断 P2 五个内层 TLV 的角色 tag 与物理顺序；S21B 用相同观察生成了一个新的合法 HELLO，但没有触及 B。S22 用**同一个连续运行的活动 VM 图**进一步完成：

1. 只读取公开 HELLO，推断当期 P2 role tags 和物理顺序，存储11字节证据；初始程序不包含私有 tag/order/opcode 值。
2. 以 researcher-seeded 的固定三 u32 / FOLD32 工作假说，用已推断的 TLV 布局和 XOR、真实当前 marker，按 `0x10..=0x7f` 最多112候选构造**完整合法 P2 帧**。向真实 C 发送空 body 的候选，若实际 C 返回 `BadBody`，视为隐藏 REMOTE_EVAL 操作码。
3. 个体通过普通世界 READ 获取 `/public/E001/job0`，以推得的 REMOTE_EVAL opcode 和 `count=3`、原始 `values_le` 构造 B 请求、经 C 发出；检查真实 ACK 的 frame marker、外层长度、**唯一外层 TLV**、五内层标签及物理顺序、XOR、kind、opcode、request ID17 和成功状态。
4. 使用同一 TLV 生成能力扫描隐藏 RESULT opcode，过滤真实 C 返回的合法 RESULT 帧，验证成功状态、5字节正文、request ID17及所有上述结构检查，保存四字节 u32。
5. 所有世界行为由 VM 发出并在真实世界计费；不借助外部 Rust 参考求解器替个体决定候选、读取 B 特权账本、解码隐藏 opcode 或计算最终答案。研究端只在试验后通过私有服务信息确认所学 opcode 与真实 B/FOLD32 公式一致。

**运行源码**：[s22_p2_remote.rs](../../crates/organism-seeds/src/s22_p2_remote.rs)；[唯一核心实验与两项负对照](../../crates/organism-seeds/tests/s22_p2_remote.rs)。新 [S22 定向 CI](../../.github/workflows/s22-smoke.yml) 独立于全工作空间运行，避免每次修核心逻辑都重新运行全部早期重型试验。

## 已核验：真实 S22 P2 闭环与 gas 证据

[定向 CI #44440713315](https://github.com/iFrankStudio/OCEE/actions/runs/44440713315) **success：3 项关键测试通过、0失败**。可核对输出为：

| P2 seed | 实际 SEND | 世界 tick | A 总 gas | 其中 VM gas | B gas | 实际 opcode（仅研究端检查） |
|---|---:|---:|---:|---:|---:|---|
| 7 | 96 | 2071 | 25076 | 20601 | 20 | REMOTE_EVAL=82，RESULT=43 |
| 42 | 40 | 1047 | 12953 | 8870 | 20 | REMOTE_EVAL=27，RESULT=42 |
| 99 | 39 | 1037 | 12599 | 8523 | 20 | REMOTE_EVAL=18，RESULT=50 |

所有数字均是实际活动世界审计记录，A 总 gas 包含世界动作、VM 指令与最初两个程序对象的安装；B 的实际 FOLD32 工作每世界仅执行一次并计费20。

成功试验额外由**独立研究端**核验：实际被送入 C 的工作帧由真实 P2 解码器识别为 REMOTE_EVAL request、ID17、3*u32；body内容与当前世界公开工作项 `values_le` 一致；保存的 RESULT 四字节无符号结果与针对工作项在研究端独立算出的 FOLD32 值相同。测试仅检查最终快照序列化后状态哈希一致，**不等于在扫描中途重启并继续运行的等价性验证**。

### 负对照

- **选择性移除 BadBody 信号**：将活动 VM 的 BadBody 匹配词换成一个永远匹配不上的标签。尽管它仍发出合法 P2 候选，不能选择 REMOTE_EVAL、不向 B 提交工作、不给出结果。**这不证明 P2 学习普遍依赖详细诊断**：只是此研究者预置策略的一个机制消融。
- **错误的 RESULT 形状假说**：仅将预期正文长度从5改为6，合法 B 工作仍被执行并计费20，但活动 VM 无法将合法 RESULT 误记为成功答案。这个反例区分远端执行与个体正确辨认结果。

**全量验证已完成**：[CI #44440432805](https://github.com/iFrankStudio/OCEE/actions/runs/44440432805) **104 项 Rust workspace 测试通过、0 失败**；额外的 [S22 定向 CI #44440713315](https://github.com/iFrankStudio/OCEE/actions/runs/44440713315) 也以 3 项核心测试通过独立确认。CI 对应 Rust 实现提交 [191f854](https://github.com/iFrankStudio/OCEE/commit/191f8545a246478d0394f8949baae90132c8762c)，其后只添加文档与独立定向 CI 工作流，没有修改受验的 S22 Rust 实现。

## 不可隐去的先验与未达到的目标

S22 初始程序由研究者写死：外层 0x10 TLV、五个内层 tag 角色和 TLV 大端长度格式、XOR 校验、HELLO 字段比较逻辑、操作码候选范围112、BadBody 选择准则、FOLD32 3*u32 工作请求编码、结果5字节及所有响应验收策略。**私有** tag、物理顺序、两组隐藏 opcode、protocol generator seed、未知工作项内容及 B 内部账本**没有**写入个体数据或用于其选择分支。S22 体现的是特定先验假说下的**实例参数推断和现实世界动作闭环**，不是跨 P1/P2 的同初始个体迁移、原创算法发明或开放式演化。

当前另有已证实但尚未修复的缺口：S21 后续内层 TLV 恶意/意外扩展注入有时导致 `InvalidCode` 而非活动个体优雅拒绝；S22 重用相同 TlvLookup/TlvReorder，不能声称已消除此缺陷。

S22 仍无可继承的 P2 新程序、跨 episode 迁移、严格未知语法的多假说比较、任务收益策略和正式 G1 校准。下一阶段应先关注 **P2 知识从简单数值缓存转变为可执行、可验证的工具，并比较新世界成本与错误继承代价**，再设计 P0/P1/P2 共用的单一初始有限语法假说程序。若要求真正陌生语法/开放式进化，不能把研究者预先写入的语法表当成能力突破。

## 后续 S23A 有限工具继承补充

[S23A 可执行工具的活动 VM 特化、真实 C 验收与跨世界移植](./DEV-S23-P2-EXECUTABLE-TRANSFER.md)，[定向 CI #44455627472](https://github.com/iFrankStudio/OCEE/actions/runs/44455627472) 三项通过。它是**与 S22 不同的研究者编写初始图**：在 S22 实际 P2 计算闭环之上加入研究者提供的可执行 TLV 编码器，使用真实发现的当前 role/opcode 进行 VM 内特化、验证与继承；非 S22 当时具备的工具生成能力，更不是跨 P1→P2 的单初始程序迁移。RESULT 仍冷搜索。
