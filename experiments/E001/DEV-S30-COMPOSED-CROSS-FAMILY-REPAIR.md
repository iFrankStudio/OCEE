# S30 开发实验协议：组合图的跨家族继承、目标侧失配修复和新输入泛化

**2026-11-20 · developmental / exploratory；未预注册 E001。科学闸门：G0 GO，G1/G2 NO-GO。**

## 继承已有成果而不重复发现

S6 已原生测试了一个研究者给定的有限语法：三个 Bytes→U32 原语 `Len`、`XorFold`、`ByteAt(0)`，两个 U32 二元运算 `Add`、`Xor`，由两次一元结果组合，共 18 个有序候选。S6 搜索真实读取公开 `/public/E001/compose` 的两组输入/输出标签，通过 `RewriteOp` 三次重写固定四节点模板，再用隔离 `EXEC` 与受费 WRITE 选择、封存/验证结果。S6 **本身不是新的算法发明**，本次直接复用其搜索内核，不把历史已通过的 S6 算作新的突破。

S29 已证明单一节点的跨协议转移和有限反馈选算子。**S30 新增**：让已有 S6 多节点搜索进入 S28 的同源 root + 同时预装 P1/P2 专家程序包；目标 P2 经两个公开组合样例重新验收来源 P1 工具；发现失配时退回同一真实 S6 搜索，然后通过有版本检查的 `WRITE(/self/tools/transform-v1, expected_version=0)` 替换旧注册对象，并不依赖宿主外部修改 active 程序。

## 世界机制与对照

1. Source P1 world seed1 和 seed10，均 protocol seed42。两者输入了**同一份冻结初始 root/hash/专家库**，通过真实活动 VM 从 18 个候选中生成多指令组合程序、封存程序与注册表、执行两个公开样例和 compose held-out。
2. Target P2 world seed19 和 seed28，protocol seed42。这两个世界对应与 source 相同的组合规则（`seed % 18`）、不同的 world seed，不能将组合函数来自研究端传给 VM。移植的只有 P1 活动 VM 真实生成的代码字节和注册引用，以两条实际受计费 WRITE 导入；目标 VM 从自身公开标签再验证，成功后实际 EXEC 未公开的新输入。
3. **新输入（OOD）**：S6 compose 的独立 held-out 原本固定，不足以证明真实跨世界输入变化；因此 S30 的活动 VM 还会读取**目标世界自己发布**的 `/public/E001/transform.held_out_input`（该字段随 world seed 变化），把新字节向量提交继承的多节点程序隔离执行，保存 U32 预测至 `/self/results/s30-extra-held-out`。研究端仅在整个世界结束后用独立的 `hidden_compose(world_seed,bytes)` 比对；不能暗中给 VM 目标答案。
4. 通过组合功能门控后，延续既有 S28 长度12判定、S26C 公开 HELLO 自主路由、真实 C→B FOLD32、独立 oracle 与世界快照。真实 B 成功属于既有专家能力链，不能将 B 的协议专家知识说成 S30 创造。
5. **配对同成本 sham**：同一 P2 world seed19，导入相同代码字节和同样大小但不可发现的 registry key；初始 root、两次受费 WRITE、资源状态与起跑 tick 一致。目标应重新运行 S6 候选搜索，若生成同哈希版本，`Conflict` 不是继承来源证据。只在成功完成 B/held-out 的两组比较净 A gas。
6. **错误语义继承修复**：将 seed10 (XOR 组合) 的工具导入 P2 seed19 (ADD 组合)，允许真实 EXEC 测试它，但它不能通过公开目标样例。个体应自主退回探索，找到适合目标的新工具并 **CAS 替换旧 registry version0→1**，然后继续两个 held-out 与真实远程计算。与 S29 的错误语义直接终止相比，这构成能力进展。
7. 每条世界都检查根程序 hash、源/目标工具因果来源、成功实际 EXEC 和预安装库一致性。形式化结论仍受研究者给定的 18 种三算子模板限制；不能由此推断未知语言的自由结构发现或任意数字世界能力。

## 反例与科学边界

- 只看相同内容哈希是不足够的；必须审计 **来源 P1 的实际版本 WRITE、目标 P2 的导入 WRITE、目标未重新生成以及成功 EXEC**。
- 若两个公开训练样例不足以唯一识别 18 种规则，目标 held-out/新输入预测可能失败；这种失败是识别性不足的重要实验事实，不得通过人为挑选更容易的隐含答案掩盖。
- 世界当前 `hidden_compose` 对空输入有边界定义，但 `ByteAt(0)` 的 VM 执行对空输入会 Range error，S6 已将其作为**未处理的全域反例**。S30 只使用当前公开 fixture 的非空输入，绝不宣称 Bytes 全域通用性。
- 任务规则与研究者模板固定；两组来源→目标只是开发 pair，未达到统计确认样本量，未预注册，仍 G0 GO / G1/G2 NO-GO。

## 工程入口

- [S30 VM 集成](../../crates/organism-seeds/src/s30_composed_transfer.rs)
- [S30 原生对照](../../crates/organism-seeds/tests/s30_composed_transfer.rs)
- [S30 专用 CI](../../.github/workflows/s30-composed-transfer.yml)
- 历史依赖 [S6 搜索](../../crates/organism-seeds/src/s6_compose.rs) / [S28 同源路由](../../crates/organism-seeds/src/s28_cross_utility.rs)。

**预注册状态：没有。所有 CI 结果需在直接获取原生日志后追加，不能按设计预期填写为通过。**


## 静态图连接失败与修正（2026-11-20）

- [S30 初次 CI #44676720789](https://github.com/iFrankStudio/OCEE/actions/runs/44676720789) 编译成功，但静态 VM `Program.validate()` 拒绝，`InvalidProgram("block parameter arity")`，没有进入学习世界；这并不说明组合算法有缺陷。
- [第一次图连接修订 #44676860088](https://github.com/iFrankStudio/OCEE/actions/runs/44676860088) 仍为同样失败。将 S28 原本作为独立 entry 的单参数块接成 `s30_after` 零参数延续是必要但不充分的修复。
- 为避免盲改，新增程序图边签名审计，运行到 [诊断 CI #44676983659](https://github.com/iFrankStudio/OCEE/actions/runs/44676983659) 精确显示两条不匹配：`s30_score1 -> apply` 和 `s30_register_ack -> apply`，实际传2、目标期待3。原因是 S6 原 `apply` 接收注册 WRITE 的第三个反馈参数，但 S30 将 WRITE 回执在 `s30_register_ack` 提前核验后，应该将 `apply` 变成只有 `hash,held_out` 两参。
- [修复 e7a9dd5](https://github.com/iFrankStudio/OCEE/commit/e7a9dd522e8ac8d313402bbe8412046d1b4102dc) 显式修改了 `apply.params`，保留全图精确边诊断。**当时定向 CI 尚未验收，不能将这些失败版本描述为通过。**


## S30 原生结果：2026-11-20 最终定向验收

- **[S30 GitHub Actions #44677074994](https://github.com/iFrankStudio/OCEE/actions/runs/44677074994) 已 completed/success**，job `native-composed-transfer`，原生 Rust **1 passed / 0 failed / 94.08 秒**；源代码检查点 [e7a9dd5](https://github.com/iFrankStudio/OCEE/commit/e7a9dd522e8ac8d313402bbe8412046d1b4102dc)。先前静态图失败的 CI 与最终通过的 CI 均已如上保留。
- 冻结完整 root 哈希：`08b80931edd47e8a0baeb34d2b448c8c3f1899577bd02fc15d144e51dfdc2459`。这一个 root、P1/P2 两份等额安装的专家库同时用于所有来源与目标世界。
- **组合样本一**：P1 world seed**1** → P2 world seed**19**（protocol seed42），由活动 VM 经真实 `RewriteOp`×3 搜索、注册和 EXEC 的工具哈希 `ccbec864ef8a002aa15e07a541cfca57200fa8770f51020274f1b1d359aa43c5`；target P2 **A 24,066 gas / 245 tick / import 29 gas / B20**，目标 VM 在切换 P2 专家之前对导入的多节点程序真实执行 **4次**（两组公开样例、组合 held-out、与来源字节不同的目标 `/public/E001/transform.held_out_input`）。
- **组合样本二**：P1 source seed**10** → P2 target seed**28**，程序 hash `81cd576411f67e85c2aa821146613e2db1f10dc5ff51e7c04ad92b12ee428c95`；目标 **A 24,063 / 245 tick / import 29 / B20**。两组来自 S6 的不同二元组合候选和不同生成版本；所有 compose-held、额外目标专属非重复输入以及真正 B FOLD32 都经独立 researcher oracle 核验正确，快照哈希重载一致。
- **等成本 sham**：P2 world19，保持相同初始 root、资源与导入时序、同一个程序内容，两次 WRITE 均导入 29 gas；可见 D **A 24,066 / 245 tick**，只改 registry 可发现性的 sham **A 24,714 / 250 tick**，D−sham **−648 A gas，约 −2.62%**。sham 自己再搜索并碰到不可变版本哈希冲突，不可将同 hash 误算为 donor 继承；成本差首先反映的是避免重复搜索，不等于不受调度影响的纯算法效率。
- **错误语义自主修复**：把 source seed10 的 XOR 类组合程序输入目标 P2 world19（需要 ADD 类组合）。目标对错误程序真实执行并由公开样例检测失配，转入真实 18 候选搜索，自主生成对应 seed1 的正确哈希，通过 `WRITE(... expected_version=Some(0))` 真正 CAS 把注册从 **v0 替换到 v1**。世界最后 **A 24,831 / 251 tick / B20**，两种 held-out 与 FOLD32 独立正确。它不是研究者替换程序，不是测试宿主强制选择；但**搜索/修复控制图全部为研究者预先编写**。
- **资源口径**：本 S30 配对统一配置 **A gas_per_epoch=100,000**；前序 S28/S29 常用 12,000。跨切片绝对 gas/ticks **不得直接比较效率**。S30 内 D/sham 使用同样初始额定配额和导入资源。
- **科学结论限度**：这是已预设 18 个有序算子组合的反馈搜索与可执行工具封存/跨家族复用，再加本研究者设计的失配后重新搜索/注册修复策略；不是开放式无界程序发明，也不是能修改自身学习算法的自主演化。来源/目标只两个开发配对，非注册的 E001 确认性 H3。G0 GO、**G1/G2 NO-GO**。最新完整 workspace CI 应单独核验，定向通过不能冒称全量通过。
