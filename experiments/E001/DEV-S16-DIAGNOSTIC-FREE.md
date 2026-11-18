# S16 — 合法响应替代错误诊断的 P2 操作码发现（开发集）

- **日期**：2026-11-18
- **代码**：[restricted_p2.rs](../../crates/calibration-solver/src/restricted_p2.rs) 的 `solve_p2_frame_only`；[协议测试](../../crates/calibration-solver/tests/p2_restricted_calibration.rs)、[实际世界测试](../../crates/calibration-solver/tests/p2_restricted_world_oracle.rs)
- **结果**：[CI #44303615806](https://github.com/iFrankStudio/OCEE/actions/runs/44303615806) 通过 76 项工作空间测试；[字节计量 CI #44304299358](https://github.com/iFrankStudio/OCEE/actions/runs/44304299358) 成功。

## 修正的科研问题

S15 对遮蔽反馈的负面对照（112 次查询后失败）将**合法帧和诊断都一起删除**。该对照仅证明 S15 的“空负载+错误码”策略在无信息输入下失效，并非“详细错误诊断不可缺”。

S16 的正面对照将每种错误诊断统一掩蔽为 `Other`，**保留正常合法协议回执帧**。求解器初始获得公开 HELLO 请求/响应并据此构造对应实例 P2 TLV 帧，但不获得隐藏 opcode、private seed、真实 B 计费或外部正确答案。探索算法是研究者编写的固定模式，不是活动 VM 自主发明。

## 具体策略

1. 推断 P2 tag 角色和字段顺序（沿用 S15 的 HELLO 双样本、已知 TLV/XOR 语法先验）。
2. 不使用 `BadBody` / `Unauthorized`，直接向 0x10～0x7f 发送**语义格式正确**的 FOLD32 工作项；仅接受校验/方向/操作码/请求 ID 均正确、单字节 body=[0] 的合法 ACK 帧。
3. 对可能的 RESULT opcode 发送空负载请求，仅接受合法 status=0 + u32 结果帧；pending 最多尝试 3 次，其余错误直接忽略。总计查询上限 226。
4. 由外部测试核验结果（独立旋转加和）、B gas 及 A/传输预算；这些特权数据不返回给求解器。

## 核验结果

协议级 12 个公开开发种子均成功，实际查询数范围 **37～151**。所有错误诊断都被掩蔽，B 只执行一次合法工作项、计费 20 gas。

真实世界：`READ(/public/C/hello)`、正常 `SEND`、等待 C→B 并 `RECEIVE`，原定 12,000 A gas/epoch。

| 开发 seed | 策略 | 查询数 | 世界 tick | A gas | B gas | A→C 发出字节 |
|---|---|---:|---:|---:|---:|---:|
| 7 | S15 错误码扫描 | 114 | 799 | 460 | 20 | 3,206 |
| 7 | S16 合法帧发现 | 95 | 666 | 384 | 20 | 3,598 |
| 42 | S15 错误码扫描 | 114 | 799 | 460 | 20 | 3,206 |
| 42 | S16 合法帧发现 | 38 | 267 | 156 | 20 | 1,232 |

**成本边界**：S16 seed 7 发送的字节反而比 S15 多，不能从查询/tick/gas 下降推导所有资源均改进。此为开发集两个特选世界，非统计证明。

**结论**：错误诊断标签不是此 P2 有界可学性案例的必要条件，但**可观察的合法响应、预置 P2 格式、FOLD32 语义、响应结构和算法均不可省略**。不是未知家族结构创建、活动 S VM 演化、E001 G1/G2 或数字生命的证据。
