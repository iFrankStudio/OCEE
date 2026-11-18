# S15 — 信息受限 P2-TLV 可学习性校准（开发集）

- **日期**：2026-11-18
- **定位**：独立的研究端参考求解器（`ocee-calibration-solver`），**不是** OCEE S/O 活动 VM 个体；非正式 E001，G1/G2 未通过。
- **首轮实测成功**：[GitHub Actions #44297484423](https://github.com/iFrankStudio/OCEE/actions/runs/44297484423)（隔离 crate 前的协议与世界交互验证）。
- **当前独立 crate 版**：[源代码](../../crates/calibration-solver/src/restricted_p2.rs)、[协议测试](../../crates/calibration-solver/tests/p2_restricted_calibration.rs)、[世界交互测试](../../crates/calibration-solver/tests/p2_restricted_world_oracle.rs)；[独立 crate 回归 CI #44298207558](https://github.com/iFrankStudio/OCEE/actions/runs/44298207558) 已通过全部 73 项 workspace 测试。

## 核心命题与关键区别

从 `/public/C/hello` 暴露的真实 HELLO 请求/响应帧，以及经过正常 `SEND(C)` / `RECEIVE` 得到的有界帧或错误诊断中，能否恢复**此协议实例**的字段角色和隐藏操作码，并实际使用 B 节点完成一次 FOLD32？

这只是证明 **P2 在已提供结构先验和受限反馈时存在有限可学路径**，不是证明个体从任意未知协议发现了通用语法；更不是自主创造编程策略。

## 输入、先验和不可访问的信息

| 类别 | S15 允许 | S15 不允许 |
|---|---|---|
| 公开观测 | HELLO 请求与响应字节、C 通过普通消息返回的帧或诊断 | 世界内部私有 `Protocol`、`Service` 字段、生成 seed、字段映射和操作码表 |
| 研究者预置知识 | P2 外层 TLV 结构、u16be 长度、TLV/XOR 校验、HELLO 请求与响应方向值、有限 `0x10..=0x7f` 操作码范围、FOLD32 工作项的请求体格式 | 正确的当期字段 tag、字段排列、私有协议种子、REMOTE_EVAL/RESULT 操作码 |
| 反馈选择 | `BadBody` 表示请求通过分帧与操作码识别但请求体不合法；`Unauthorized` 表示未创建工作项时查询结果 | 真实 B gas、外部 evaluator 得分、已知答案或后台服务状态作为求解器输入 |
| 计费审查 | 外部测试在运行**后**读取 B 侧账本、世界审计链、A gas 消耗 | 通过这些数据指导候选生成或在求解器中决定是否成功 |

独立 crate 的普通 `[dependencies]` 为空。模拟服务、世界内核和 `serde_json` 仅是 `[dev-dependencies]`，其构造器出现在研究端测试适配层而非求解器实现，防止误将私有参数读路径带进算法。

## 算法行为（全部为研究者设计，不属于个体自主发明）

1. 校验 HELLO 请求/响应完整封装及内外 TLV 长度；利用同一 tag 在两个报文中的相对行为推断 request-id、body、checksum、kind、opcode 的角色。
2. 从 checksum 约束识别校验 tag，从请求/响应方向差异识别 kind；恢复实例字段排列与已公开 HELLO opcode，要求能重新构造完全相同的合法 HELLO 请求。
3. 对 `0x10..=0x7f` 的 112 个候选操作码，以同一 request-id 发送符合已推断 P2 结构的空负载请求，只以可见诊断区分 REMOTE_EVAL 与 RESULT。
4. 用发现的远端计算 opcode 发送真实三元素 FOLD32 任务；使用发现的结果 opcode 取回输出，最多轮询 3 次。
5. 外部测试比较独立旋转加和参考值，同时确认 B 计算实际收费一次、A→C 符合资源与通信限制，世界快照重放一致。

**不会**向活动 S 程序安装研究端求解器，且不允许将参考求解器结果、字段映射或操作码写入个体初始 manifest。

## 开发测试覆盖

- 协议级：公开 seed `0,1,2,3,4,5,7,17,42,99,2026,65535`，覆盖不同 P2 标签与顺序，验证有限查询、真实远端结果与独立参考值。
- 负面：截断报文、请求/响应 marker 不一致、两次请求误作 HELLO 配对会拒绝推断。
- 世界级：seed 7、42，使用 `Action::Read` 读取公开 HELLO，`Action::Send` 提交请求，等待实际 C 解析及延迟回复，用 `Action::Receive` 获取结果；审计 A 在单 epoch 12,000 gas 内、B 计费 20 gas、确定性快照复验。
- 12 个种子是**开发集**；不能据此计算正式成功率，也不能替代 >=8 独立配置的完整 G1 校准协议。世界级仅覆盖两组开发种子，且远程任务长度固定为三个 u32。

## 尚未跨越的闸门

1. **初始活动 VM 没有执行 S15 的发现算法**；参考求解器还未移植为无宿主帮助、正常计费的 S 程序。
2. 求解器明确知道 P2-TLV/XOR 框架、错误码和 FOLD32 请求体。更陌生的语法、P1→P2 结构迁移和泛化学习尚未测试。
3. P2 的两份公开 HELLO 示例本身具有强信息量；要做对照消融，评估示例不足/诊断关闭时可学习性如何变化。
4. 未建立正式 E001 的 A/B/C/D 四组策略、长期任务收益、独立 S/O manifest 冻结、校准/确认 seed 隔离和预注册。

**结论边界**：S15 可以成为 G1 信息受限参考求解器的一个实测子能力，但不是 G1 通过，更不是 E001 M0 或赛博生命出现的证据。
