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
