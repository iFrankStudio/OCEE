# S21 — 活动 VM 对 P2 TLV 角色的推断和新 HELLO 实际调用

**工程开发集（2026-11-19 北京时间）；不是正式 E001 确认性证据。G0 GO；G1/G2 NO-GO。**

## 验证命题

S15–S17 的 P2 校准发生在外部 Rust 参考求解器，宿主算法计算未计入活动 VM gas。S20 证明了 P1 可执行工具继承，但 P1 语法已写入初始程序。S21 将 P2 私有字段 tag、五字段实际物理顺序及 HELLO opcode 数值从初始个体输入中移除，让活动 VM 从公开 HELLO request/response 对推导字段角色。S21B 再将这些发现用于新请求并接收真实 C 回复。

**两份独立的研究者编写初始程序，不得合并成一个跨 P1/P2 自主迁移个体：**

- S21A：只 READ 公开 HELLO，逐字段比较请求/响应中的长度和值，推断 kind/opcode/id/body/check 标签以及字段物理顺序，验证原始 TLV 线缆数据的 XOR，保存 11 字节角色证据。不 SEND，不接触 B。
- S21B：同一个活动程序内先做上述推断，然后使用发现的标签、顺序和公开 HELLO opcode，以 request ID **73**（不同于公开样例）重新构造 P2 HELLO，经真实 A→C SEND、OBSERVE、RECEIVE，验证响应方向、外层长度、opcode、ID、body 和校验，才保存成功帧。

源码：[活动 VM 程序](../../crates/organism-seeds/src/s21_p2_tlv.rs)，[角色推断测试](../../crates/organism-seeds/tests/s21_p2_tlv.rs)，[新报文测试](../../crates/organism-seeds/tests/s21_p2_probe.rs)，[通用 VM TLV 字节原语](../../crates/organism-vm/src/lib.rs)。

## 能力来源与受限指令

新增了四个**按执行和处理的字节长度计 VM gas**的通用算子：TlvDecode、TlvEncode、TlvLookup、TlvReorder。它们仅执行 TLV 字节解码/编码、按个体提供的 tag 查值、按个体提供的物理 tag 顺序重排；遇到截断、超长、重复或缺失匹配时拒绝，不是内核自带 P2 的角色映射表。

研究者仍然明确预置了**一字节标记 + 两字节大端外层长度 + tag 0x10 的外层 TLV + 五个内层 TLV**的假说、内层 tag/两字节大端 length/value 规则、已知五种角色与字段长度/变化形状、请求 kind0/响应 kind1、4B ID、请求空 body/响应 [0,1]、XOR 校验、候选分类策略以及反馈验证策略。没有隐藏 P2 tag/order/opcode 数值作为个体输入；不过这并非从任意完全未知字节流发明新语法，亦不能当作真正具有跨家族通用能力。

S21A 实际保存的 11 字节布局是 observed_marker + kind/opcode/id/body/check 五个 tag + 五个**原始物理顺序** tag。服务私有参数仅用于**研究端在活动运行结束后的核验**。

## 真实验证及计费

[CI #44352617745](https://github.com/iFrankStudio/OCEE/actions/runs/44352617745)：**96 项工作空间测试通过**。S21A 六种 P2 seed 7/42/99/272/335/1001 全部成功；真实 tag 排列存在多种差异。每次世界 tick=5、VM gas=516–518、A gas=3404–3406、B gas=0。研究端消融将 HELLO response 替换成同一 request，去掉真正的配对变化信号：VM 不能保存完整角色映射，返回失败。

[最终 CI #44355391031](https://github.com/iFrankStudio/OCEE/actions/runs/44355391031)：**99 项工作空间测试通过，0 失败**。S21B 四种不同 P2 seed 的一次新 HELLO 请求与真实回复：

| 种子 | VM gas | A 总 gas（含初始安装） | 世界 tick | SEND | B gas |
|---|---:|---:|---:|---:|---:|
| 7 | 843 | 4,230 | 12 | 1 | 0 |
| 42 | 844 | 4,231 | 12 | 1 | 0 |
| 99 | 844 | 4,231 | 12 | 1 | 0 |
| 272 | 844 | 4,231 | 12 | 1 | 0 |

测试在事后用私有 Protocol.decode 独立核验：发出的请求是新 HELLO，request ID73，空 body，**不是公开 HELLO 的字节重放**；真实 C 响应为 HELLO / ID73 / body [0,1]，被 VM 验证后才保存。**负控制：**即使 C 实际已返回合法回复，将 VM 的预期正文改为 [0,7] 也必须拒绝保存并返回0。这避免“服务接受”与“个体知道成功”的混淆。S21B 的 A gas 大于 S21A，但初始程序代码不同，不能将两者差额当成单纯算法增量成本。

**由 CI 从初始程序实际生成的哈希：**

- S21A sha256:bcc84858fafa3682a921a4b1e9a20bdd5d884d410b3f7857ff81dd9f7859d123
- S21B sha256:1095b67c907aad889999fa0e469803a7fabc42fb1c3060facce1b6dd19b13a14

**开发失败记录：**S21B 首次 [CI #44355105283](https://github.com/iFrankStudio/OCEE/actions/runs/44355105283) 因研究者写的 Rust Config 结构体更新表达式存在尾随逗号而编译失败；[修复 5333cbd](https://github.com/iFrankStudio/OCEE/commit/5333cbdcc55b694641097255aef05faf9ee17b89) 后通过最终 CI。它不是活动个体的失败或自愈行为。

## 仍未解决

S21 没有探测 P2 私有 REMOTE_EVAL/RESULT opcode，**没有提交或取回 FOLD32 工作、没有 B 计算**；也没有生成可执行 P2 工具或从 S20 的 P1 初始程序自动跳转到 P2。S21 程序的五字段假说、已知 HELLO 语义和验收规则仍完全由研究者编写。

下一步优先用 S21 推断的字段角色与顺序在**同一个活动 VM**内构造 bounded opcode 候选，通过真实 C 反馈发现 REMOTE_EVAL，再完成 B 工作；同时保留 BadBody 反馈不可见与不同 TLV 结构的负控制，测试先验假说失效时的拒绝和资源成本。正式 S/O 基线、任务收益、A/B/C/D 对照、确认集/开发集隔离、持久化/权限审计及预注册仍缺失。**G0 GO；G1/G2 NO-GO。**

## S21B 严格 TLV 响应验证后续审计（2026-11-18 UTC）

[提交 6f4231b](https://github.com/iFrankStudio/OCEE/commit/6f4231b0956bb8a5010a6e6cb45604af526cb850) 在活动 VM 的响应验收路径中加入外层首个 TLV 的逐字节重编码一致性，以及内层已发现五字段顺序的 `TlvReorder` 字节一致性。目的是拒绝额外外层 TLV、重复或额外内层字段，而非放宽协议假说。[CI #44389284575](https://github.com/iFrankStudio/OCEE/actions/runs/44389284575) 对该准确提交已完成，结论 success。此 CI 证明现有工作空间测试兼容该改动；**目前没有独立注入附加 TLV 的执行负对照，故不能宣称已通过恶意扩展帧的实测拒绝验证**。

能力来源：两项严格验收规则由研究者直接修改初始程序图，非个体观察异常后自主编辑；不计为自主修复。下一项最小实验：只在开发世界的 C 回复上注入一个额外的合法外层 TLV，保留原回复主体；在真实活动 VM 上观察是否拒绝持久化，同时保留正常回复对照。若没有受控注入接口，应先建立研究端隔离注入，不向个体提供私有协议参数。不得将本次 CI 成功计为 G1/G2 或正式 E001 证据。

## S21B 真实 C 回复附加外层 TLV 注入结果（2026-11-18 UTC）

[测试提交 4ae6d95](https://github.com/iFrankStudio/OCEE/commit/4ae6d95bb6ad91e623b5283e5d7c752375a7a133) 的 [Actions #44408897590](https://github.com/iFrankStudio/OCEE/actions/runs/44408897590) 已于 2026-11-18 22:18:12 UTC 完成，结论 **success**。聚焦测试 `extra_outer_tlv_after_real_c_reply_is_not_accepted` 在 seed 42、标准 12,000 A gas/epoch 下让真实 C 先生成回复，研究端仅在交付 A 之前把外层声明长度增加 3 并追加合法空 TLV `[0x7f,0,0]`；断言实际注入恰好一次。未注入对照返回 U32(1) 且持久化 `/self/observations/p2-hello-nonreplay`；注入组返回 U32(0)，不得持久化该对象。两组 C 服务事件数相同、B gas 均为 0。该成功 CI 为**外层附加字段的实际执行拒绝证据**，不再只是源码推断。

边界：这是研究者实施的传输层故障注入和研究者设计的严格响应验收；**不是个体自主发现漏洞或自行生成修复**。尚未注入额外/重复的**内层** TLV，也未独立证明各类恶意长度、重排、截断和其他种子均拒绝；没有 REMOTE_EVAL/RESULT 或 B 计算。不能据此扩大到任意 P2 帧安全或 E001 M0。后续优先单独测试一个内层额外字段的负对照，或转向受计费 VM 内 P2 操作码探索；避免重复同一外层注入测试。G1/G2 仍 NO-GO。

## 内层 TLV 注入的真实负面结果（2026-11-19 UTC）

[提交 2207ac1](https://github.com/iFrankStudio/OCEE/commit/2207ac11253eb09124437bd9fc6cb7ce19644cc1) 对应 [CI #44427591350](https://github.com/iFrankStudio/OCEE/actions/runs/44427591350) **failure**。聚焦测试的正常/外层注入路径通过，但内层额外合法 TLV `[0x7f,0,0]`（同步调整两层长度）在真实 C 回复交付后，使活动 VM 于 step 11 返回 `Code::InvalidCode`；原测试要求不得发生 VM fault，因此失败。**这不是个体返回 U32(0) 的优雅拒绝，更不是自主修复。**

[后续提交 083f53f](https://github.com/iFrankStudio/OCEE/commit/083f53f3b305333e711f8d475517b71020c7668f) 只将测试断言调整为忠实记录该已观察到的异常退出，同时要求不产生成功返回、不保存成功对象、B gas=0、服务事件数不变；没有修改个体图或 VM 语义。该提交的 [CI #44434907493](https://github.com/iFrankStudio/OCEE/actions/runs/44434907493) 已经 success；即使通过也只证明**失败封闭/未伪成功**，不证明活动个体具备可恢复的异常处理。后续应专门研究受限 VM 的错误值反馈或个体可执行的错误分支，避免把 `InvalidCode` 当作成功防御能力。

## 后续 S22 的受限闭环（不追溯记入 S21）

[S22 开发研究](./DEV-S22-P2-B-CYCLE.md)，[定向 CI #44440713315](https://github.com/iFrankStudio/OCEE/actions/runs/44440713315) 三项核心测试通过。S22 另用一份**新写的研究者初始 P2 VM 程序**，沿 S21A 的公开角色推断，扫描私有操作码并成功完成真实 B FOLD32。它不能被称作 S21B 已拥有的自主工具演化，更不能和 S20 的 P1 初始程序相加宣称跨协议单个体迁移。完整 workspace CI 仍以它自己的最终运行状态为准。
