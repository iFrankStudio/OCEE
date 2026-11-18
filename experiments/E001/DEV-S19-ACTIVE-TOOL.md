# S19 — 活动 VM 的 P1 可执行工具特化、采用与跨世界复验（开发集）

2026-11-18。**研究阶段；正式 E001 尚未启动；G1/G2 仍 NO-GO。**

## 背景和可证伪命题

S18 已证明活动 VM 能在预先写入的 P1 报文结构先验下，通过合法世界动作查出 REMOTE_EVAL/RESULT、调用 B 并保存正确结果。但 S18 **没有创建新的程序版本**，B 请求完全由其初始程序中的固定指令组生成。

S19 试验两个不同层次的命题：

1. **工具特化**：同一活动 VM 在看到 C 的真实反馈后，能否以自己发现的 opcode 和 HELLO-derived XOR key 改写一份研究者预置的可执行编码器模板，在只读隔离 EXEC 中校验其正确字节行为，保存内容寻址版本并通过注册引用实际使用？失败候选即使 EXEC 成功也不能采用。
2. **跨世界继承**：全新世界只收到上一轮已经注册的不可变程序字节和工具引用，活动 VM 是否能自己用合法 C 诊断检查兼容性？匹配时避免昂贵的重新搜索，不匹配时回退并 CAS(0) 更新旧引用？这些都是开发集可检验机制，不是 E001 正式 C/D 组。

## A. 已完成并验证：同一活动 VM 创建并采用工具

源码：[s19_p1_tool.rs](../../crates/organism-seeds/src/s19_p1_tool.rs)，测试：[s19_p1_tool.rs](../../crates/organism-seeds/tests/s19_p1_tool.rs)，提交 [fb6650a](https://github.com/iFrankStudio/OCEE/commit/fb6650aba8238005cd811f3388ed043e86f48af9)；[CI #44330835759](https://github.com/iFrankStudio/OCEE/actions/runs/44330835759) **成功，84 项 Rust workspace 测试通过/0 失败**。

- 活动 VM 保留 S18 的 P1 HELLO/诊断和 112 候选扫描流程，发现远端操作码后，先写入发现证据。
- 初始个体有一份**明确研究者提供**的 P1 编码器 AST 模板，两个带类型的常量节点对应“请求操作码”和“当期 XOR key”。活动 VM 通过 `RewriteOp` 先后特化两处结构，重新计算规范字节与 `OCEE-PROGRAM-V1\0` 域分隔 SHA-256 哈希，写到 `/self/versions/<hash>`。
- 使用 `WorldCall::Exec` 对一个不同于正式工作项的固定三元素输入做隔离自检，检查 `code==Ok` **仍不算成功**：还必须从实际 `EXEC` 响应解析 Bytes，与另一组 VM 指令计算的预期编码帧逐字节比较。通过后才绑定 `/self/tools/p1-eval-frame`。
- 必须通过注册程序的普通 `EXEC` 生成实际工作项帧，再由活动程序 `SEND` C、获取 ACK、继续 S18 的 RESULT 搜索、取回真实 u32。种子 7/42/99 通过，B 各计费 20；测试审计真实 `EXEC` 两次（自检 + 工作项）、实际 SEND 与工具输出相同、最终值符合独立 FOLD32 算式。
- **负控制**：编码器模板的帧分隔符被人工故意写错，但程序类型正确且隔离 EXEC 返回 Ok。VM 的独立字节比较拒绝采用，不注册工具，不向 B 提交工作。

### 必须拒绝的推断

全部 P1 framing、XOR/escape、已知输入布局、112 候选范围、实例参数的替换节点、参考编码器、自检门槛，以及何时注册的策略**都是研究者显式写入初始 S19 程序的先验**。它产生了一个参数已特化、功能被复验的真实可执行程序，但**不等于创造新算法、发明陌生语法或一般性编码知识**。与最初的 S3 P0 模板特化同属有限模板变体；本轮新事实是 S18 发现与 P1 真实结果闭环的直接、计费衔接。

## B. 跨世界继承与负迁移：已通过完整 CI

源码：[s19_p1_transfer.rs](../../crates/organism-seeds/src/s19_p1_transfer.rs)，测试：[s19_p1_transfer.rs](../../crates/organism-seeds/tests/s19_p1_transfer.rs)，提交 [54f187f](https://github.com/iFrankStudio/OCEE/commit/54f187fe80e83372925b0820c127241af642be06)；[最终 CI #44335557937](https://github.com/iFrankStudio/OCEE/actions/runs/44335557937) **87 项 Rust 工作空间测试通过 / 0 failed**，覆盖 S19 正常、同 P1 实例继承、不兼容 P1 实例回退和 S19 中途程序版本快照恢复。

- 使用与冷启动完全相同的 S19 transfer-aware 初始程序；新世界的研究端驱动器只通过普通 `WRITE` 搬运旧世界中该个体拥有的不可变工具程序与 `/self/tools/p1-eval-frame` 哈希，不复制 continuation、邮件、上次任务输出、世界隐藏状态或计算资源余额。
- 新世界内活动 VM 首先 `READ` 现存工具指针，`EXEC` 输出带真实学习 opcode 的 **4-byte request_id、空 body** P1 探针，再由真实 C `SEND/RECEIVE` 测试。`BadBody` 说明该帧在当前实例中已经被识别为 REMOTE_EVAL 且校验/结构通过，才允许复用该版本构造工作项；它不是一般语义正确性的完整证明。
- 未获合法 `BadBody` 则回退 S18→S19 的活动搜索与工具自检、使用 CAS(0) 替换继承的旧引用。探针自身不应调用 B；只有真正三元素 FOLD32 成功调用 B20。
- 开发世界实际完成：donor 的 `config.seed=42/protocol_seed=42`，新 target 的 `config.seed=128/protocol_seed=42`，具有**不同的公共工作项**但相同协议实例；真实 `EXEC→SEND→RECEIVE` 合法 BadBody 探测后复用原工具。负迁移 donor 为 `config.seed=272/protocol_seed=272`，目标仍为 `128/42`；旧工具由活动 VM 实测拒绝、完成冷搜索并 CAS(0) 替换工具引用，最终同样得到 B 的独立参考数值。
- CI 工作流调至 20 分钟，以覆盖 Rust 工作空间的资源计费长周期回归；改用 `cargo test --workspace -- --show-output` 在**同一轮测试运行**输出成本记录，避免为了日志重复执行所有沉重实验。

## C. 接下来仍缺失的关键机制

- 继承验证仍使用**研究者写死的 P1 空负载探测以及 C 的 `BadBody` 错误标签**。一个拒绝/接受信号不足以检验未知程序在整个输入域的泛化；跨语法 P1→P2 不适用。
- P1 编码器只封装合法请求字节，不含 B 任务语义、自主收益函数或协议发现过程。当前仍需要预置搜索器去推断 RESULT，不能视作可携带到任意软件/互联网的完整技能。
- 正式 S/O 初始能力的冻结、四组机制对照、开发/正式 seed 隔离、更多预算和陌生协议校准、事务安全审计及预注册仍未完成。**G0 GO，G1/G2 NO-GO**。

## D. S19 最终计费与结果（复现实验，非 E001 确认性证据）

[最终 CI #44335557937](https://github.com/iFrankStudio/OCEE/actions/runs/44335557937) 使用 `cargo test --workspace -- --show-output` **单次执行全部 87 项测试**，没有为了输出计费记录而重复跑历史 S14–S18 各套测试。两条跨 episode 场景均使用同一 S19 transfer-aware 初始程序、每 epoch A 配额 12,000，B 仅在真实三 u32 FOLD32 工作请求时计 20 gas，快照可重放。

| target 世界 (config.seed=128 / protocol.seed=42) | 全生命周期 A gas | 世界 tick | 导入 A gas | B gas | 终态 |
|---|---:|---:|---:|---:|---|
| 冷启动无继承 | 6,669 | 130 | 0 | 20 | 正确 |
| 继承兼容 P1 工具 | 6,046 | 129 | 62 | 20 | 正确 |
| 继承不兼容 P1 工具并回退重学 | 7,070 | 139 | 62 | 20 | 正确 |

相对于**同一个 S19 transfer-aware 冷启动版本**，兼容继承在 target episode 节省 623 A gas（9.34%），tick 只减少 1；不兼容继承额外付出 401 A gas（6.01%），tick 增加 9。**这不是跨全生命周期累计节约**：donor 的初始学习费用另计；也不能据此证明 S19 比**不生成工具的 S18**净资源更优。两种工具都只覆盖 P1 特化模板，而非 P1→P2 的结构迁移。

**冷启动 S19 工具构造本身的费用**（源于同一最终 CI）：seed 7 / 42 / 99 的 A gas = 32,635 / 6,181 / 23,515，tick = 2,305 / 128 / 1,366；每个工具程序 860 字节，isolated EXEC gas 总计 74，B 各 20。相比 S18 对应开发 seed 的 A gas 31,446 / 4,998 / 22,318，新增工具特化、自检、落盘及初始程序增量的成本约 1,200 A gas。不能隐去一次性成本后计算复用投资回报。

初始程序能力来源（由代码 manifest 自动计算，开发记录使用，**未正式冻结为 E001 S**）：
- S19 active P1 specialization: `sha256:914bf7806d48ea77c8fd8c982640d7e583172e23cef656e7dde6770e19546474`
- S19 transfer-aware active P1: `sha256:2a01b1dbc0399beec271ed0363b35290568c54d55cc10554b4907f6a041e9f34`

### 故障与修复记录（保留失败证据）

- 初始迁移控制流从 `Branch` 直接跳入需要 `Yield` 反馈的五参数块，静态 VM 校验拒绝。CI #44332968604 失败；在 [eea3bf1](https://github.com/iFrankStudio/OCEE/commit/eea3bf153bb175bd1c8755e679234a17c6a17bdb) 加入真实且计费的 OBSERVE 反馈桥，而非关闭类型检查。
- CI #44334557967 的两条工具继承测试已经通过并产生上述成本，但整套 CI 因另一路 S19 快照回归在固定 10 tick 假设“仍未注册工具”而失败；seed 42 实际更快完成初期发现。 [395eb6d](https://github.com/iFrankStudio/OCEE/commit/395eb6d0cb663b22c7177cfa626174e1c7092e4d) 改为观测**已写第二个不可变版本而尚未注册工具**的状态后快照；最终 #44335557937 验证状态恢复与独立无重启执行完全相同，B 仍仅计 20。
- **两种故障是开发程序和回归检查点错误，不应记作个体自身的发现、恢复或演化。**

## E. 下一阶段问题

当前跨 episode 重用只避免了 REMOTE_EVAL 的扫描；RESULT 仍通过 S18 的 112 候选逐项探索，解释了兼容组仅节省 623 gas、1 tick 的事实。下一优先方向是由活动 VM 同时封装已学会的 RESULT 操作码/解析器并跨 episode 验证，核验完整积累曲线；然后推进没有完整研究者预置专用解析器的 P2 结构假说搜索和异构迁移。继续严格保持 **G0 GO、G1/G2 NO-GO**。
