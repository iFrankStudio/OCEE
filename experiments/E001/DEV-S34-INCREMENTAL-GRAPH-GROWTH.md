# S34：由活动 VM 逐步拼装类型化程序，而非在完整拓扑模板间选一套

**2026-11-20；开发性机制实验。G0 GO；G1/G2 NO-GO。非预注册 E001 H3。**

## 研究问题及比 S33 更进一步之处
S33 证明两个研究者完整预建的四指令/六指令图可被 VM 按公开标签选中并跨 P1/P2 直接参与真实 B 计算。但实际上仍然是选择完整模板，不是由个体拼装新程序阶段。S34 保留一个共同的、包含三次 RewriteOp 插槽的 S6 四指令骨架，研究者只提供有限类型安全的阶段片段。VM 本身读取公开样例决定 0、1 或 2 次附加阶段；借助新通用 `AppendInstructions(program,block,serialized Vec<Instruction>,return_id)` 实际**修改已验证程序结构**并设定新的 return 寄存器。不是宿主在启动前安装三个完整程序。

## VM 新原语的安全约束
- 原语是通用型的、属于研究者提供的计算基础设施，不能解释成个体自己发明了 AppendInstructions；其四个参数分别为内容寻址代码字节、区块名称、序列化追加指令向量以及结果寄存器 U32。返回新的规范序列化 Program Bytes。
- 仅允许操作已有 **Return** 区块，需先验证输入程序合法、规范序列化无差异；附加内容与修改返回值后必须再运行完整 Program.validate，确保寄存器标识/类型/控制流/字节上限/指令上限合规。绝不绕过世界资源/权限或 VM 隔离。
- 与 RewriteOp/PrependBlocks 一样，新原语的 VM gas 成本按原程序长度和追加片段长度计量，并由 world 统一扣费。独立原生 VM 单元检查：正确追加改变实际计算；错误类型、重复寄存器、未定义返回 ID 均拒绝；空追加且原结果不变必须保持原字节/哈希一致。

## 研究者先验与公开任务
- 研究者提供 Len、XorFold、ByteAt 三个一元原语和 Add/Xor 两个二元原语、循环/采样/验收策略，18种底层组合；还提供可重复长度增量片段 `Len(input); Add(previous,Len)`。片段依旧以研究者定义的两阶段具体寄存器布局表示，可追加 0/1/2 次，开放的是已有图上的追加组合，而不是任意深度自动图发明。
- 沿用已白名单允许只读的 `/public/E001/compose-depth` 开发 fixture 路径，由实验世界宿主在初始化阶段插入**两条公开标注输入/输出**，不提供 rule/seed/深度标签。隐藏参考规则为 `hidden_compose(seed,input) + floor((seed%54)/18)*Len(input)`，深度0/1/2；仅研究者用于生成公开输出和在完整执行后比较隐藏预测。源 world seed 1/19/37，目标 seed55/73/91；每对 seed%54 相等，因此功能相同、公开任务与额外新输入可以不同。
- 冻结初始 root 必须在六个世界完全相同，同等预装 P1/P2 专家和 S31 的独立长度工具。根 VM 使用受限54候选搜索，每个尝试经真实三次 RewriteOp、一次 AppendInstructions、新版本 WORLD WRITE、两个样例 EXEC 检测，只有样例正确才注册其内容哈希，并将该程序用于新世界字节输入，结果成为真正的 B FOLD32 第四个操作数（S32 因果路径）。

## 原生实验与消融
- **三种深度 P1→P2**：P1 seed1→P2 seed55 选0额外阶段（4指令）；P1 seed19→P2 seed73 选1阶段（6指令）；P1 seed37→P2 seed91 选2阶段（8指令）。目标重新验收实际继承程序的两个公开样例，利用从来源不同的、未标注字节实际 EXEC 所选程序生成第四操作数，使用 S31 计数工具和两份已有协议专家真实请求 B；校验真实 C wire、B 账本、独立 fold32、快照和内容哈希。不能用仅相同 hash 推断迁移。
- **深度上限消融**：研究者将最多 2 阶段缩至最多 1 阶段、真实穷尽前36个候选；面对 seed91 的二阶段规则，不能产出真实第四操作数，也不能凭其它专家完成 B。必须检查实际版本写入的候选数量恰好36，排除非搜索原因的失败。
- **等付费假注册表**：正常继承与同样付费导入同样字节但只改不可见 registry key；两者应该都能完成真实 B，正常继承少于 sham 的 A gas。不能把假对照中重搜出的相同 hash 算成迁移。
- **错误学习工具恢复**：将来源 seed19 单阶段工具导入需二阶段的 P2 seed91；目标真实 EXEC 两样例检测错误，回退搜完整54候选，CAS registry v0→v1，然后生成正确第四数完成真实 B。恢复策略是研究者早前编写的 S30 状态机，而不是新学习算法。

## 风险及科学结论限度
训练样例目前仍仅两条，可能有别的候选在样例上巧合通过；因此所有正例必须核验实际选择深度以及目标独立输入，不能改动隐藏规则或人为挑例后淡化失败。新 AppendInstructions 原语是**研究者提供的图编辑能力**；组合阶段 bank 的长度最多2，递归扩展/自生长未被验证，未接真实互联网服务，未做大样本。即使通过也只能说有限增量程序图构造及跨家族功能迁移，**不能声称开放式程序发明、自主演化或通过 E001 G1/G2**。

代码：[S34 生成器](../../crates/organism-seeds/src/s34_incremental_depth.rs)；[S34 原生世界对照](../../crates/organism-seeds/tests/s34_incremental_depth.rs)；[VM 新通用编辑器](../../crates/organism-vm/src/lib.rs)；[S34 专用 CI](../../.github/workflows/s34-incremental-depth.yml)。

**原生 CI 结果须从实际日志追加。首次全量编译暴露 VM 新单元测试遗漏本地 literal 构造器，已作为工程失败记录保留并修订；不可当作个体学习结果。**


## 2026-11-20 原生验收追加（与研究预案分开）

- 修复提交 [fa45749a9cce826f86b5f0f0fdb532b7bd7edde0](https://github.com/iFrankStudio/OCEE/commit/fa45749a9cce826f86b5f0f0fdb532b7bd7edde0) 将世界 WRITE 的程序版本字节从追加前寄存器 224 改为实际 `AppendInstructions` 生成的寄存器 245，使内容寻址哈希与不可变存储字节一致。此前 [#44692967512](https://github.com/iFrankStudio/OCEE/actions/runs/44692967512) 失败；这属于工程集成缺陷，不是个体自主修复。
- [S34 定向 Actions #44693172156](https://github.com/iFrankStudio/OCEE/actions/runs/44693172156) 对应上述 SHA，已核实 **completed/success**。同一 job 的「Type-safe incremental graph VM primitive and rejection tests」与「Native 0-1-2-stage construction, P1-to-P2 real B and strict negative controls」两步骤均为 success。由此确认在固定开发 fixture、预置片段和有界候选空间内，原生测试所断言的三种深度、跨家族 B 路径、深度上限负例、等付费 sham 和失配后重学均通过；**尚未从工件独立提取每组 A gas/tick 数值，不在此杜撰数值或泛化显著性**。
- 同 SHA 完整 workspace [#44693172200](https://github.com/iFrankStudio/OCEE/actions/runs/44693172200) 查询时仍为 in_progress；**定向通过不能替代完整回归**。后续先验收完整工作流，再提取定向原生打印的各组资源账本；若有失败，按 job/日志定位，不重复启动已有 CI。
- 能力归因保持保守：研究者预置的 `Len;Add` 追加片段、0/1/2 阶段、18 底层组合、两条标签、P1/P2 专家及 S32 远程 ABI 均为先验。新验证的是**活动 VM 受限编辑并保存实际可执行的不同长度图**，而非自行发明新的编辑原语、任意拓扑或开放式演化。正式 E001 未预注册；G1/G2 继续 NO-GO。

## 2026-11-20：原生实测及工程失败记录

- **最终定向原生 Actions [#44693172156](https://github.com/iFrankStudio/OCEE/actions/runs/44693172156) 已 completed/success**；首先运行 VM 编辑器单元测试（**1 passed、0 failed**），随后 S34 原生世界实验（**1 passed、0 failed、126.57s**）。获验收的 Rust 源码 SHA [fa45749](https://github.com/iFrankStudio/OCEE/commit/fa45749a9cce826f86b5f0f0fdb532b7bd7edde0)。完整 workspace [#44693172200](https://github.com/iFrankStudio/OCEE/actions/runs/44693172200) 仍需单独核验，不可以定向测试冒称全量通过。
- 固定、与家族和 seed 无关的初始完整 root SHA256：`18e02a10bb3d641701bf5fa7908cd4b1d1f2ae15056dc7ec794e1cf6c203a5b8`。研究者共用初始四指令骨架、18种原始算子组合、两种延伸阶段片段（stage1：Len(input)+Add(acc,len)，stage2：再追加一次同构阶段）；VM 按 `candidate_index % 18` 和 `candidate_index / 18` 从同一个静态有限语法派生规范程序。核心与 S33 的区别是使用 **新增通用 VM AppendInstructions 原语** 在运行中延伸原有代码并修改 Return，而不是选择研究者预制的三个完整程序。
- **0 阶段**：source P1 seed1→target P2 seed55，真实生成4条指令程序 hash `ccbec864ef8a002aa15e07a541cfca57200fa8770f51020274f1b1d359aa43c5`，受费导入 **29 A gas**，来源 **A13,526**，目标 **A24,605 / 250 ticks / B24**；目标继承/复验/未见输入 EXEC/生成真实第四个 FOLD32 操作数都完成。
- **1 阶段**：source P1 seed19→P2 seed73，真实在原图追加2条指令、6条指令程序 hash `ba26376fb3a4613f0f0090125ef1c8349993c965087792c068a302b3a143745b`，import **32 A**，来源 **A22,886**，目标 **A24,620 / 250 ticks / B24**，真实 C/B 和独立结果通过。
- **2 阶段**：source P1 seed37→P2 seed91，原图追加4条指令、8条指令程序 hash `b66d0084a3106dafbee2ae2426404c5d6c016c7016a08baf23ff13e920647931`，import **35 A**，来源 **A32,889**，目标 **A24,625 / 250 ticks / B24**；测试检查最终8个指令 id/算子及真实 C 帧与独立 B FOLD32，绝不是只认同一 hash。
- **结构上限负对照**：将同一搜索限制为最多1阶段，仅允许先前 **36种**候选；面对需要2阶段的 P2 seed91，原生测试确认**36个候选全部通过实际 WORLD WRITE 产生尝试**，仍 **B0 / A26,135 / 80 ticks**，没有伪造可执行操作数；不是初始权限异常或提前终止误判。
- **等起点同导入收费的可见/不可见对照**：两臂 P2 world91 使用完全相同的 root、导入版本字节、两次受费 WRITE 及起始时序，import 各 **35 A**；可见继承 **A24,625 / 250 ticks / B24**，不可见 registry 键的 sham **A44,926 / 327 ticks / B24**，D−sham **−20,301 A gas（约45.19%的 sham 总 A）**。成本差主要来自避免自行重新遍历36种浅层/一阶段错误候选，不代表未知任务上的普遍收益。
- **错误学习拓扑的目标自主恢复**：把只增加1阶段的来源 P1 seed19 工具错误注册给必须增加2阶段的 P2 seed91。目标根据自己的两条公开标注输入/输出拒绝继承程序，真实运行54候选搜索，生成正确的8指令版本，经 **CAS registry version0→version1** 把错误引用替换，最后正常计算第四操作数并完成真实 B，**A45,019 / 328 ticks / B24**；完整恢复策略和循环逻辑是研究者预置的 S30 机制。

### 必须保留的失败及调试链
- 早期新 VM 单元测试漏写 `literal` 局部工具导致 Rust **E0425 未编译**（例如 [全量 #44692178156](https://github.com/iFrankStudio/OCEE/actions/runs/44692178156)、[定向 #44692468446](https://github.com/iFrankStudio/OCEE/actions/runs/44692468446)）。[dc1eaac](https://github.com/iFrankStudio/OCEE/commit/dc1eaac2cad8081d42e054153966c5a42be29a0e) 只补齐 VM 单元测试内的字节字面量生成器，不改变待验证研究机制。
- 接下来 [定向 #44692580672](https://github.com/iFrankStudio/OCEE/actions/runs/44692580672)、[带精确 VM continuation 的 #44692967512](https://github.com/iFrankStudio/OCEE/actions/runs/44692967512) 进入真实世界，但在第19个候选（第一次追加一阶段）遇到 **InvalidCode**。诊断证实 VM 已成功执行追加且计算新 hash，却错误地把追加前寄存器224的规范代码字节提交给 `/self/versions/<追加后hash>`，触发**应有的世界内核内容寻址校验拒绝**。修订 [fa45749](https://github.com/iFrankStudio/OCEE/commit/fa45749a9cce826f86b5f0f0fdb532b7bd7edde0) 使 WORLD WRITE 真正写入追加后的寄存器245，与哈希一致。没有放宽校验、降低例子难度或改变原生 B 服务。
- VM 通用编辑器独立测试在通过版确认：追加 `Len`、`Add` 后新程序的类型与真实数值正确；重复 register、类型错位及无定义返回寄存器被拒绝；空追加与原原程序保持完全相同内容字节。编辑器始终对输入/输出代码进行完整静态验证并按程序+片段大小收费。

### 结论与限制
**可支持的结论**：在研究者预先定义、最大2阶段的有限增量语法内，活动 VM 能利用通用的计费/类型化图扩展操作从一个基础骨架生成4/6/8条指令的真实程序，根据两条公开标签选择、跨 P1/P2 继承/验证/修复，且程序输出实际决定远程 B 的一个计算操作数。**不能据此宣称**任意程序图自由生长、跨任意任务零样本算法发明、资源预算自主探索、真实互联网接入、赛博生命产生或通过预注册 E001 G1/G2。G0 GO；G1/G2 NO-GO。下一步建议 S35 提高动态可组合的阶段上限/拓扑变化并研究 sample alias、额外未知输入与自主预算分配，同时维持真实 B 因果检验。
