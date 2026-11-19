# S23A — 活动 P2 VM 可执行 REMOTE_EVAL 工具的特化、验收与跨世界复用（开发集）

**日期：2026-11-19（北京时间） · G0 GO；G1/G2 NO-GO。** 不是正式 E001 预注册结果。定向测试已通过；完整 workspace 回归以单独 CI 为准。

## 对 S22 的真实增量

S22 只在活动 VM 内保存 P2 role map、REMOTE_EVAL 与 RESULT 操作码的证据；每个新世界都须重做 112 候选的远端探索。S23A 让**同一活动 VM**把自己从公开 HELLO 推得的11字节 role map 与真实 C 的 BadBody 发现的 REMOTE_EVAL opcode 写入研究者预置的通用 P2 TLV 编码器 AST，在隔离 EXEC、真实 C 的空请求 BadBody 验收后才以 CAS 注册为不可变工具，然后仅由这份工具的实际 EXEC 输出完成真实 B 的 FOLD32 工作帧。RESULT 仍用 S22 已知 P2 假说的冷扫描。

新世界最初安装**完全相同**的 S23A 活动程序。研究者迁移驱动器仅通过普通、受计费的 WORLD WRITE 导入上一 episode 的不可变工具 program 字节与 `/self/tools/p2-eval-frame` 内容哈希，不复制旧 continuation、pending feedback、工作结果、服务事件、账本、候选状态或世界 RNG。活动 VM 先 READ 注册工具，隔离 EXEC 构造空 body 候选，实际发 C，只有收到合法 `BadBody`（当前服务确实识别为 REMOTE_EVAL 且结构、长度和 XOR 合法）才继承使用；否则返回原来的112候选完整探索、生成新程序、真实 C 复验并 CAS(0) 覆盖失效引用。导入工具并不自带信任。

实现：
- [研究者预置 P2 编码器 AST](../../crates/organism-seeds/src/s23_p2_tool.rs)
- [活动 VM 特化、版本化、验收、继承与回退图](../../crates/organism-seeds/src/s23_p2_transfer.rs)
- [三项针对性因果及计量测试](../../crates/organism-seeds/tests/s23_p2_transfer.rs)
- [独立定向 CI](../../.github/workflows/s23-smoke.yml)

## 已通过的计费实验

[定向 CI #44455627472](https://github.com/iFrankStudio/OCEE/actions/runs/44455627472)：**3 passed / 0 failed**。工作空间全量 CI [#44455627462](https://github.com/iFrankStudio/OCEE/actions/runs/44455627462) 在本记录初次归档时仍待确认。被测目标环境为 world seed128、P2 protocol seed42，初始 S23A 活动程序一致；兼容 donor 的 world seed42、P2 protocol seed42；失效 donor 的 world seed7、P2 protocol seed7。私有 seed 仅供研究端建立评估场景，不交给活动 VM 选操作码。

| 新世界策略 | A gas（实际总额，含导入与初始程序安装） | tick | 导入 gas | B gas |
|---|---:|---:|---:|---:|
| 冷启动，两操作码均重新探索、生成工具 | 19,712 | 1,132 | 0 | 20 |
| 继承兼容的 REMOTE_EVAL 可执行工具，RESULT 仍冷探索 | **14,452** | 1,057 | 102 | 20 |
| 继承失效的 REMOTE_EVAL 可执行工具，真实 C 拒绝后重新探索、CAS替换 | 20,319 | 1,142 | 102 | 20 |

兼容继承相对于**同一 S23A 初始程序冷启动**节省 **5,260 A gas（26.68%）**，减少75 tick；失效继承相对冷启动增加 **607 A gas（3.08%）**，增加10 tick。两项导入均已经计费。冷启动另测 world seed=protocol seed=99，**19,277 A gas，1,127 tick，B20**，成功。研究端独立核验：真正发 C 的 B 工作报文与注册工具对当前公开 `values_le` 输入的隔离 EXEC 输出逐字节一致；P2 decode 为 REMOTE_EVAL ID17、count3；保存的四字节结果等于独立 FOLD32；各 episode B 只计费一次 20。

**资源解释**：A 每 epoch 授予12000 gas，所有这些新世界都跨过至少一次补充边界，所以相对于大幅 gas 减少，世界 tick 的收益小得多。生成、两次 RewriteOp、哈希、存储、隔离 EXEC、C 验收、导入、等待和失败重试均实际计入 A 账本。**不能把只继承一个工具的阶段解释成 S20 P1 双工具复用那样已经避免 RESULT 扫描**，更不能将重复开销从对照中扣掉。

## 真实开发故障与修复

1. 首版 S23 与 S21 的五份展开式字段识别图相加超过 VM 128 基本块上限；试行144后又碰到64KiB程序限制。[初始失败 CI #44453262447](https://github.com/iFrankStudio/OCEE/actions/runs/44453262447)、[第二失败 #44453907578](https://github.com/iFrankStudio/OCEE/actions/runs/44453907578)。最终没有上调任何受限 VM 上限：**S23 专用图**把五份相同的 S21 字段识别块重构为一次 RepeatBounded(5) 的通用字段分类循环，输出相同信息，基础 VM 恢复原128块、64KiB、4096指令、有限循环和类型校验。
2. 工具验收与共享发送/接收图产生静态潜在回边；[CI #44454252686](https://github.com/iFrankStudio/OCEE/actions/runs/44454252686)、[#44454413694](https://github.com/iFrankStudio/OCEE/actions/runs/44454413694) 观察失败。用 `RepeatBounded(1)` 对“发现操作码后首次构造工具”与“工具验收后提交真实任务”两次阶段迁移设置边界，**没有关闭无界环检测**。
3. [CI #44454579203](https://github.com/iFrankStudio/OCEE/actions/runs/44454579203) 两组继承测试通过，但 seed99 冷启动在第1028步收到合法 `RECEIVE=Empty`，旧图直接 JsonIndex 空 payload 而 `InvalidCode`。使用调试信息定位具体反馈块，再以现有 `RepeatBounded(128)` 控制有 gas 成本的 OBSERVE→RECEIVE 重试，只在动作状态 `Ok` 时解析 payload；超过上限则进入失败分支。[最终定向 CI #44455627472](https://github.com/iFrankStudio/OCEE/actions/runs/44455627472) 三项通过。修复改变真实 A 成本与 tick，故不沿用修复前数字。
4. S21 已记录的**注入多余合法内层 TLV**可能触发 `InvalidCode` 而不是优雅拒绝，S23 的新 Empty 重试不等于解决 TLV 解析异常；这仍是独立缺口。

以上开发错误、诊断和修改全部是研究者工程过程，**不计作个体自身发明代码、修复 VM 或自主进化**。

## 信息来源、研究先验与未完成能力

预置内容仍包括：P2 单外层五字段 TLV/XOR 框架、字段长度与差异判别、112操作码范围、BadBody 操作码选择、固定 ID17/3u32 FOLD32 任务、RESULT 响应校验、完整可执行帧编码 AST、两个 `RewriteOp` 插入位、依 C 反馈决定注册/回退的控制流。活动个体只是在**已知语法假说**下发现当前实例角色和 opcode，用真实世界反馈检验特化程序；不是从零发明了新语法、推导任意程序逻辑，也不是 S20 P1 的一个个体自动迁移到 P2。

当前仅封装 REMOTE_EVAL，不包含 RESULT 作为独立可执行继承工具，未知新语法结构变化/畸形字段可恢复性、三个以上 P2 实例稳健性、多 episode 全生命周期成本摊销、正式初始 S/O 冻结、C/D 清洁重置和四组对照、开发/确认集隔离及预注册都未完成。**G0 GO，G1/G2 NO-GO。**

## 后继 S24 容错演化边界（不归功于 S23 初始个体）

[后续 S24 总函数 TLV 验证与受损 ACK 的结果恢复](./DEV-S24-RECOVERABLE-TLV.md)，[CI #44502050065](https://github.com/iFrankStudio/OCEE/actions/runs/44502050065) 7项定向通过。S24 将此前可能引发 VM InvalidCode 的额外内层 TLV 改成可正常拒绝；一个独立研究者预置的恢复程序可跳过不可信 ACK，查询并验证真实 RESULT。**这不是 S23A 个体自我发现修复**；原 S23A 继承收益与研究者 P2 先验归因保持不变。
