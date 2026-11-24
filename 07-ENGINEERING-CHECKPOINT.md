# OCEE-0 工程检查点：P1/P2/P3 初始实现

- **日期**：2026-11-19
- **状态**：工程原型可运行，**E001 尚未校准、未预注册、未产生科学实验结果**
- **最近已确认全量 CI**：[GitHub Actions #44519428660](https://github.com/iFrankStudio/OCEE/actions/runs/44519428660) — Rust workspace **113 passed / 0 failed**（S24 最新成功基线）；[S25 定向 #44524418058](https://github.com/iFrankStudio/OCEE/actions/runs/44524418058) **1 passed / 0 failed**；S25 全量 [#44524418089](https://github.com/iFrankStudio/OCEE/actions/runs/44524418089) 待最终验收。
- **适用基线**：[04 工程契约](./04-ENGINEERING-CONTRACTS.md)、[05 个体基线](./05-INITIAL-ORGANISMS.md)、[06 实验实现](./06-E001-IMPLEMENTATION.md)

## 已实现并由 CI 验证的能力

### P1 — 世界内核（初始可运行切片）

- 逻辑 tick、环境事件先行、epoch 授予、受限资源账户、延迟消息队列、A→C 可达、A→B 直连禁止。
- 世界快照与还原、稳定有序状态、SHA-256 状态摘要和事件审计链。
- 带版本的本地对象、CAS 与存储配额原子拒绝。
- **未实现** SQLite 事务持久化、完整权限与跨内核版本规范序列化；当前 JSON 快照主要用于实验原型。

### P2 — 可修改个体 VM（初始可运行切片）

- 类型化程序图、字节计算、算术与 SHA-256、规范程序哈希、静态验证和 gas 限额。
- 有界 `RepeatBounded` 循环（每个头最大 256 次）、循环计数在 continuation 中持久化；任意无界 CFG 环仍被禁止。
- 活动程序发出单项世界操作意图，反馈在后续 tick 恢复执行。
- 不可变候选程序写入、CAS 切换活动版本、历史版本回退；隔离 `EXEC` 支持纯计算和**已授权本地快照只读**，禁止外部写入和消息探测。
- 目前原型 `EXEC` 世界动作能够接受受限输入字节；S1/S3 在 VM 内通过内容寻址程序版本，生成并自行采用一个固定算法模板衍生出的工具。该工具的结构来自人工先验，不能算真正的新算法发明。
- **未实现** 跨函数 `CALL`、完整 `Unit/Result<T>` 类型、开放式图组合搜索；当前已有通用单节点 `RewriteOp` 和有限候选选择示例，但这些搜索策略来自研究者初始程序。自修改端到端测试所用候选由研究者事先构造，**不是自主涌现**。

### P3 — 模拟异构协议与远程服务（初始可运行切片）

- `ocee-protocol-sim`：P0（marker/length/XOR）、P1（escaped delimiter/XOR）、P2（嵌套 TLV）的种子确定性编码、解析与错误处理。
- C 节点接收请求并调用 B 的唯一有界运算 `FOLD32`，B gas 实际扣除；`REMOTE_EVAL` 和 `RESULT` 按请求编号关联，具有幂等性与异步可见语义。
- C→A 反馈通过定时消息返回，并扣除通信预算；不使用真实网络或宿主命令。
- 每 256 个逻辑 tick 在 `/public/C/hello` 发布一对合法 HELLO 帧，供个体通过世界 `READ` 观察；不可通过 READ 获得私有协议生成参数。
- 隐藏协议参数目前存于世界权威状态。协议实验夹具以研究者视角构造合法请求用于验证服务，**还不是个体自学协议**。
- 新增有限公共反馈探测研究夹具：给定 HELLO 样例、已知 P0/P1 通用分帧规则和可见错误码，能够在有限请求内发现 REMOTE_EVAL opcode；这**不是**从零推断所有协议规则，也不覆盖 P2，不能作为真实 S 型学习结果。
- 已有开发用固定 3 个 u32 的可见 FOLD32 工作项，存储于 `/public/E001/job0`。S2 通过读取该工作项构造 P0 请求并实际调用 B，随后用有限探测发现 RESULT 操作码，收到并保存远程结果；见 [S2 开发记录](./experiments/E001/DEV-S2.md)。这不是正式工作项发布与评分系统。
- **未实现** E001 正式任务调度与评价与统计、服务版本变化事件、真实 S/O 初始程序、独立受限参考求解器、真实可学性校准。

### S0/S1 — 个体行为起步（开发集证据，非 E001 正式结论）

- S0 是由研究者注入的最小 VM 程序，通过 `/public/C/hello` 观察、发送公开合法 HELLO 样例、轮询反馈并自主保存到 `/self/observations/*`。P0/P1/P2 × 两个种子均在 CI 中通过。
- S1 是更具针对性的**P0 已知 XOR 分帧先验**程序。它不知道隐藏 opcode，逐一构造候选、依据 `BadBody` 错误反馈选择并保存 `/self/discoveries/p0-operation-candidate`；两个开发种子下通过。
- 控制组关闭诊断选择机制，同样尝试 112 个候选但不保存发现。该对照是研发机制消融，**不替代** E001 的 A/B/C/D 实验组。
- S1 使用 `100000 A gas/epoch` 的开发额度，而不是正式 E001 资源配置；不能把这两次成功视为在预定稀缺资源下已成立。
- S0/S1 都不生成新可执行程序，也不证明未知 P1/P2 完整语法推断、远程任务成功或跨 episode 能力迁移。详见 [开发实验检查点](./experiments/E001/DEV-S0-S1.md)。

### S2 — 已知家族内从发现到远程计算结果（开发集）

- S2 利用 S1 保存的 P0 REMOTE_EVAL 候选，结合世界公开的工作项生成正确 FOLD32 请求；B 消耗 20 gas（3 个 u32 的开发任务）。
- 个体使用已赋予的 P0 帧格式知识，通过错误反馈与响应长度/校验信息发现 RESULT 操作码，读取并保存 `/self/results/job0/value_le`；两种开发种子的结果由独立算式交叉验证。
- S1→S2 的程序切换由研究者执行，不是个体自创程序；尚未证明累积演化、自主创造工具或跨协议家族迁移。

### S3 — VM 内生成可执行工具（开发集，模板特化）

- [研究事实记录](./experiments/E001/DEV-S3-SPECIALIZATION.md)：S1 的扩展个体无需人工阶段切换，自行生成并试运行工具版本、注册工具、使用它构造公开工作项请求，并收到 C/B 回执。
- P0 三个开发 seed（7、42、99）通过；同一生成工具可处理 1/4 u32 两种不同长度的测试工作负载，B 计费分别 12/24 gas；无诊断反馈时工具不能生成。
- 生成程序的帧构造结构仍然由研究者作为模板片段提供；**不计为新算法、新能力语义或 M0 达成**。

### S4 — 结构性修复的最小可检验证据

- VM 新增通用 `RewriteOp`：能基于规范化程序 JSON 修改任意指令节点，重新验证类型和控制流，正常计费；不会自动选择任务策略。
- [S4 开发记录](./experiments/E001/DEV-S4-STRUCTURAL-REPAIR.md)：错误的 Len checksum 候选能合法执行，但功能验证失败，个体才在有界循环内改为 XorFold 后注册并实际调用 B；开发种子 7、42 通过。
- 关闭结构改写的消融无工具产生；另有 S3 错误校验和负对照确保 `EXEC Ok` 不被误判为工具功能成功。
- **P0 专用假说、操作符集合、比较规则仍为初始先验**；这是一次有界一节点结构修复，不是开放式程序合成，也不是 M0 达成。

### S4 标准资源预算下的可行性诊断（仅开发集）

- [详细数据](./experiments/E001/DEV-S4-RESOURCE-CALIBRATION.md)：种子 7、42、99、2026，在标准 A gas 每 epoch 12,000 和开发 A gas 100,000 两种配置中均完成了 S4 工具生成与 B 任务；均保留失败计数。
- 标准配额下完成 tick 介于 246～1,237；3 个种子遭遇 666～673 次 gas 拒绝。开发配额下为 246～571 ticks，均无 gas 拒绝。标准实验中的资源停滞主要来自外部 epoch 刷新，不等同个体学会主动资源调度。
- 初始程序安装成本也占用第一个 epoch 配额，但没有计入自主 tick 的 VM/世界费用统计；经济性评估必须计入完整生命周期。
- 这些种子全部是开发种子，不能用于 G2 正式集或推断总体成功率。

### S5 — 数据样例指导的封闭词表结构搜索（辅助研究）

- [S5 开发实验记录](./experiments/E001/DEV-S5-OPERATOR-SEARCH.md)：世界公开两组标注输入输出以及一个无标签保留输入，隐藏参考算子留在世界/研究端。
- 一个不随规则切换的 VM 搜索个体，在 `Len/XorFold/ByteAt` 词表内生成多个类型化可执行候选，用 `EXEC` 与训练样例反馈选择、注册并在保留输入上执行；六个开发种子覆盖三种目标算子，均通过。
- 对照组只有首个候选：遇到 `Len` 成功，其余两种规则失败。所有测试使用原定每 epoch 12,000 A gas。
- **词表、骨架、试验次数与样例比较器全部预置；该辅助任务未调用 C/B，也不属于 E001 的协议跨家族适应证据。**

### S6 — 固定语法中的多节点组合（辅助研究）

- [S6 研究记录](./experiments/E001/DEV-S6-COMPOSITION.md)：复用 S5 的 VM 内监督式样例选择路径，将候选从单节点扩大为两个一元节点与一个二元组合节点（18 种候选）。
- 全部 18 个隐藏规则配置（其中存在语义等价类）通过非空保留输入核验；固定第二个一元节点为零的机制消融无法解决需要两种输入特征的组合任务。
- 新增 `Div(U32,U32)` 用于通用候选索引分解；世界单独发布 /public/E001/compose 的两个标注样例和无答案保留输入。仍不依赖 C/B 服务。
- **组合节点与候选集合是研究者事先提供的有限语法**，并非新原语创造、策略自发成长或 E001 G1/M0 通过；S6 主实验最初使用 100,000 A gas/epoch，后续三种子标准预算校准已完成，但不是正式 G1 校准。

### S6 标准配额校准与边界反例

- [研究记录](./experiments/E001/DEV-S6-COMPOSITION.md)：三组公开开发种子（0、7、16）在每 epoch 12,000 与 100,000 A gas 配置下均完成，tick 8 / 18 / 36，全部无 gas 拒绝。原定配额尚未成为本次小型组合搜索的瓶颈。
- 全部 18 种隐藏组合配置的成功工具又各经独立四组非空未见输入验证；但空输入违反部分生成工具的隐含前置条件，触发 `VmError::Range`，虽参考函数对空输入存在定义。已以负面测试封存，不得宣称全类型域泛化。
- `EXEC` 动作次数包含候选试验、第二样例验证和保留输入，不可直接作为候选数量报告；[S6 资源报告](./crates/organism-seeds/examples/s6_calibration.rs)已按真实计数语义更名，较重校准转为[按需运行](./.github/workflows/s6-calibration.yml)。

### S7 — 已采用工具故障后的受限控制流修复（辅助研究）

- [S7 研究记录](./experiments/E001/DEV-S7-BOUNDARY-REPAIR.md)：同一活动 VM 在完成 S6 有限搜索/注册后自主测试空字节输入；三个开发 seed（2、5、16）观察到实际隔离 `InvalidCode`，记录故障，并用通用图插入在已注册工具前添加空输入分支，独立验证和 CAS 采用。
- 新旧程序版本均保留、活动初始程序未由研究者切换；错误补丁返回 1 的对照即使程序可执行仍被拒绝，禁止修复的对照保留故障状态，无故障 seed 0 不发生多余修改。
- VM `PrependBlocks` 操作重验证全部图与类型，保持无界 CFG 环拒绝和 gas 计量；相关安全不变量已有最小测试。
- S7 的正确零值假说、守卫块结构和空输入探测策略仍为人工预置；后续 seed 2 在原定 12,000 gas/epoch 下通过针对性测试，但仍未完成分阶段 gas 统计、跨 episode、P1/P2 及正式对照校准。

### S8 — 故障后有限修复候选选择（辅助研究）

- [S8 证据](./experiments/E001/DEV-S8-REPAIR-SEARCH.md)：首次候选程序合法可执行、但空输入返回 1；个体从环境观察到实际输出不符合预置目标 0 后拒绝，再在同一 VM 的受限 RepeatBounded 内生成第二版，执行回归测试并 CAS 采用。
- 三种开发 seed（2、5、16）在原定每 epoch 12,000 A gas 配额下通过；禁止第二次尝试时无法修复；无故障 seed 0 不发生无谓修改；S7 seed 2 在标准配额下也通过。
- **返值词表 [1,0]、正确值 0、控制流守卫与搜索顺序都被研究者预置**。这是有限候选范围的反馈选择，不是正确修复算法的自主发现。完整资源成本、跨 episode 和正式 E001 仍未验证。

### S8 标准资源预算全生命周期计量（开发集）

- [CI #44228823269](https://github.com/iFrankStudio/OCEE/actions/runs/44228823269)：**47 passed / 0 failed**；[资源计量代码](./crates/organism-seeds/examples/s8_cost.rs) 覆盖 S8 seed 2/5/16、S7 seed 2、S8 单候选失败和无故障对照。
- 三个 S8 成功 seed 的安装至终止总 tick **24/30/48**，累计 VM+世界动作 gas **3,597/4,671/7,865**，存储 **20,530/21,533/24,512 字节**；全部 **0 gas 拒绝**，存储上限 262,144 字节。
- 同 seed 2 的 S7→S8 多候选试验额外成本：**+684 gas、+1,942 字节、+2 tick**。此为具体实现比较，不是统计推断；未衡量跨 episode 复用收益和正式任务价值。
- 详细原始数据、口径与局限见 [S8 研究记录](./experiments/E001/DEV-S8-REPAIR-SEARCH.md)。

### S9 — 环境边界见证指导修复与证据不足时拒绝采用（开发集）

- [S9 研究记录](./experiments/E001/DEV-S9-BOUNDARY-WITNESS.md)；[CI #44236131831](https://github.com/iFrankStudio/OCEE/actions/runs/44236131831) **成功：49 项 workspace 测试通过，S8 计量和 manifest 通过**。
- 原工具实际空输入 `EXEC InvalidCode` 后，世界才发布模拟的外部边界见证。相同 VM 初始程序在两次一致证据为 0 或 1 时生成、验证并采用相应守卫补丁；证据缺失、相互矛盾或关闭见证读取时保留旧工具，保存未解决状态。
- 采用标准 12,000 A gas/epoch 和 256 KiB 存储；测试涵盖 seed 2、5、20、23、38、41 及 seed 5 的无见证消融，快照重放与非空回归核验。
- **能力来源限制**：见证的目标由研究者控制的世界模拟提供，正确性不由个体独立发现；候选集合 [1,0]、守卫语法和选择策略仍预置。两个一致观测不代表可信。此结果仅支持有限外部监督下的条件修复/拒绝决策，**不证明新算法发明、开放式演化或 E001 M0**。

### S9 — 故障后模拟外部见证与不确定性处理（开发集）

- [S9 研究记录](./experiments/E001/DEV-S9-BOUNDARY-WITNESS.md)：初始 VM 不再包含正确目标为 0 的比较器。世界仅在已注册工具真实空输入执行失败后发布公共边界见证，两份同值观察引导修复；无证据、冲突或禁用见证则明确保持未解决、不 CAS 更新工具。
- 在开发 seed 2/5/20/23/38/41 下覆盖目标 0/1、缺失和冲突；原定 12,000 gas/epoch 下正常完成。CI #44253475161 **58 passed / 0 failed**，并打印 S9 初始程序哈希。
- **重要负面结果**：seed 17 的初始工具对空输入正常执行却返回与参考规则不符的数值；由于没有执行错误，故障触发的外部证据不会发布，S9 未发现静默语义错误。不能把“不会抛异常”解释为“输出正确”。
- 两份观察均出自研究者定义的同一模拟世界函数，**不是独立、可认证的真实证据**；完整程序骨架和两候选列表仍是人工先验，不支持独立算法发明、正式 E001 或 G1/G2 达成。

### S10 — 在未报错时主动检验语义正确性（辅助研究）

- [S10 开发实验](./experiments/E001/DEV-S10-SEMANTIC-PROBE.md)：同一初始 VM 在 seed 17 遇到 `Ok(0)` 的**静默错误**（参考为 1），主动读取已执行工具的模拟边界见证、隔离复查、记录实际返回值，再生成及 CAS 采用正确修复；这突破了 S9 只在运行故障后才取证的限制。
- seed 2/5 运行故障仍可修复；seed 0 已正确则不变；seed 20/23 无效或冲突证据不采用；禁止语义反馈时 seed 17 继续保持错误结果。按原定 12,000 A gas/epoch 测试通过。
- S10 世界报告现在要求关联**真正执行**（Ok 或 InvalidCode）的注册工具调用；gas 被拒绝时不产生报告。CI #44253475161 **58 passed / 0 failed**，含专门安全回归。
- **测试输入空串、世界的语义正确性标签、两个同源见证、两类补丁及候选结构全部预置**。此能力不代表开放式自主语义发现、不意味着能识别伪造的见证；G1/G2 仍 NO-GO。

### S11 — 双模拟证据通道交叉核验（辅助研究）

- [S11 研究记录](./experiments/E001/DEV-S11-CORROBORATION.md)：世界在真实的注册工具 `EXEC` 后分别发布已有的模拟见证与独立代码实现的第二核验值；活动个体对第二来源做任务/输入范围、标签数量和值匹配检查，只有一致才进行原工具比较、候选修复和 CAS。
- seed 2/5/17 的正确模拟来源支持修复，seed 0 无需修改，缺失或冲突不采用。关闭第二来源也不采用；**污染单一来源**会被拒绝，**同时污染两来源**仍使个体采用语义错误的版本，作为明确负面结果保留。
- [CI #44253475161](https://github.com/iFrankStudio/OCEE/actions/runs/44253475161) **58 passed / 0 failed**。原有资源拒绝安全断言已扩展至第二通道，并在 #44253475161 通过：未实际执行时不得泄露隐藏答案。
- 两个参考函数是**同一模拟世界及 seed 控制的代码实现**，而非真正独立且认证过的现实来源。测试输入、候选集合、比较与采用策略均为人工先验；无来源身份协议、可信度推断、跨 episode 或未知 P1/P2 学习证据。

### S12 — 模拟证据的来源声明与程序版本绑定（已通过 CI）

- [S12 开发记录](./experiments/E001/DEV-S12-PROVENANCE.md)：为两条模拟证据增加来源声明及受检程序哈希，并让活动 VM 在使用证据前验证；添加来源污染、跨版本替换、机制消融与双来源共同伪造的聚焦测试。
- [最终 CI #44257611715](https://github.com/iFrankStudio/OCEE/actions/runs/44257611715) **60 passed / 0 failed**；跨程序替换对照使用了**实际保存在版本库内的另一份合法程序哈希**，而非仅仅一个格式错误的字符串。
- 正常种子仍能完成修复和拒绝；源角色标识或受检哈希不符时拒绝 CAS，移除验证机制后不符哈希依旧可能被误用。两个通道同时伪造元数据与数值依然无法保证真实性；同版本重放也未校验。
- **元数据声明并非来源签名；所有额外判断和有限修复语法仍由研究者赋予。**这是程序上下文绑定，不是可信认证或 E001 M0。

### S13-A — 个体生成的探针挑战（并行开发路线）

- [S13-A 研究记录](./experiments/E001/DEV-S13-CHALLENGE.md)：个体从受限 `OBSERVE` 反馈生成并保存探针挑战，在自己发起 `EXEC` 后逐项核对两个世界报告的挑战回显；真实旧运行中相同程序/输入的报告无法通过本次不同挑战核验。关闭挑战检查则会接受重放。
- [同一最终 CI #44264595858](https://github.com/iFrankStudio/OCEE/actions/runs/44264595858) 的工作空间 66 项测试包含此路线的正常/重放/消融/共同伪造对照。其初始哈希与下述 S13-B **不同**；不能把两者拼成一个已验证的个体。
- 挑战来自公开世界可观察状态，**不具备密码学不可预测性**；同时伪造两份报告仍可导致错误采用。探针输入、挑战生成和校验规则均是研究者先验。

### S13 — 精确 EXEC 反馈绑定与同版本重放（辅助研究）

- [S13 研究记录](./experiments/E001/DEV-S13-EXEC-RECEIPT.md)：世界对实际 `EXEC` 输出增加可选的 `exec_step`（仅 Ok/InvalidCode，不包括 gas 拒绝）；个体自行保存那次工具执行反馈，然后将两条模拟报告的 `observed_after_step` 与该执行序号比较，失败则记录 `replayed_execution` 且不采用。
- 开发 seed 5/17 的严格重放控制：**同一已注册程序、相同空输入、第一次真实 EXEC 生成合法旧报告，然后个体第二次真实 EXEC**；S13 拒绝旧报告，S12 机制消融却接受，隔离了新鲜度绑定的因果作用。
- 正常修复与无效监督的保守拒绝依旧成立；两个模拟报告若由特权测试夹具同时改写成**本次执行编号下的一致假值**仍会诱导错误采用，是必须保留的负面结果。
- [最终 CI #44264595858](https://github.com/iFrankStudio/OCEE/actions/runs/44264595858) **66 passed / 0 failed**。当前逻辑步回执只在 u32 有效范围发行，报告没有签名；所有挑战策略和修复候选仍由研究者预置，**不能推断独立来源认证或 E001 M0**。

### S14 — 跨 episode 自建工具继承、验证与重新学习（开发集）

- [S14 工程记录](./experiments/E001/DEV-S14-EPISODE-TRANSFER.md)；[已通过 CI #44266619063](https://github.com/iFrankStudio/OCEE/actions/runs/44266619063)，**68 passed / 0 failed**。
- episode 1 的活动 S14 VM 在 S6 封闭词表内自行生成并注册可运行工具。下一 episode 是全新世界/协议实例与资源账户，研究者的 episode 驱动器通过普通受计费 `WRITE` 只搬运已注册的不可变程序和工具指针，不搬运 continuation、结果、邮件、权限或隐藏世界种子。
- 两个 episode 语义一致但世界 seed 不同（17→35，均 `seed%18=17`）时，同一初始 VM 在新世界验证两个有标签的公开示例后直接复用旧工具；与新世界从零开始的同程序对照相比，计入新世界初始安装和导入后的 gas **1,144 vs 4,691**、tick **9 vs 30**，导入成本 29 gas；新世界末存储 **14,499 vs 17,607 字节**。
- 不同语义（0→17）时，活动 VM 识别已有工具与新公开样例不相容，重新执行受限 S6 搜索，注册新工具并使用 CAS(0) 替换旧工具指针；成本 **4,884 vs 4,691 gas**、**34 vs 30 tick**，显式记录负迁移而非只挑选胜例。旧程序仍保留，不等于世界宿主救援。
- **严格边界**：迁移驱动器是研究者编写的；并非 VM 自己决定 world reset；迁移内容仅限两类明确对象，两个公开样例可能无法排除过拟合，所有表达式与回退策略仍为 seeded。既没有跨 P1/P2 未见协议家族，也不是 E001 A/B/C/D 完整实验或 M0。


- **S14 三 episode 追加 CI 复核（2026-11-18）**：[提交 b981a09b](https://github.com/iFrankStudio/OCEE/commit/b981a09b464a52676b96b5b8e3b7ed97e1fc892d) 对应 [Actions #44274013758](https://github.com/iFrankStudio/OCEE/actions/runs/44274013758) 已完成成功；S14 三项测试通过。17→35→53 同规则链，**包含首轮学习的累计 gas 为继承 6,979 / 全冷启动 14,073**，两次导入各 29 gas；约节省 50.4%。详见 [S14 记录](./experiments/E001/DEV-S14-EPISODE-TRANSFER.md)。此为开发集特选同语义路径，不抵消异规则负迁移，G1/G2 仍 NO-GO。

### S15 — 公开 HELLO 推断 P2 字段角色与操作码（研究端有限可学习性校准）

- [S15 详细记录](./experiments/E001/DEV-S15-P2-RESTRICTED-CALIBRATION.md)；[成功 CI #44298207558](https://github.com/iFrankStudio/OCEE/actions/runs/44298207558)：Rust workspace **73 passed / 0 failed**。
- 独立的 `ocee-calibration-solver` 库本身**没有生产依赖**；权威协议模拟器与世界内核只能在测试适配层作为 dev-dependencies 使用。参考求解器仅接收公开 HELLO 字节对和 `Frame` / `BadBody` / `Unauthorized` / `Other` 可见反馈，不能调用隐藏种子、生成器、服务内部对象、B gas 或外部参考答案。
- 12 个公开开发 seed，在 P2 基础 TLV/XOR 结构先验和有界 opcode 范围下，通过样例推断实例 tag 的角色和排列，经有限反馈查到 REMOTE_EVAL 与 RESULT，调用 B 的三元素 FOLD32 并正确取回数值。
- seed 7、42 的 world-level 测试从合法 `READ(/public/C/hello)` 开始，通过 A→C 的真实 `SEND` / 有延迟的 `RECEIVE` 调用，单 epoch 12,000 A gas 内完成；研究者在结束后审计 B 计费均为 20 gas、独立参考数值相等、世界快照回放一致。
- 负面样例（截断、marker 不符、错误 HELLO 配对）被拒绝。**未知 TLV 字节边界自身、诊断含义、三 u32 负载格式、以及诊断扫描策略均属于研究者先验**。这只是 G1 独立受限校准求解器的一个必要子能力，**不是活动 VM 自主推断协议、正式跨家族迁移或 M0 达成**。

### S16 — 屏蔽错误标签但保留合法帧：P2 可学性对照

- [S16 研究记录](./experiments/E001/DEV-S16-DIAGNOSTIC-FREE.md)；[已通过 CI #44304299358](https://github.com/iFrankStudio/OCEE/actions/runs/44304299358)，全部 76 项 workspace 测试通过。
- S15 的“诊断屏蔽”把**全部反馈**都替换为 `Other`（包括正确帧），只能表明那种扫描策略无法工作，**不能**推出诊断标签必要。S16 改为向 112 个候选 opcode 发送结构正确的三 u32 工作负载；在**所有错误标签均被掩蔽**时识别合法 status=0 接收回执，再用具有 u32 结果的合法响应找到 RESULT。
- 12 个 P2 开发 seed 均能远端求值，最多 151 次查询（设定上限 226），B 每次任务只计费 20 gas。真实世界 seed 7：S15→S16 查询 114→95、A gas 460→384、发送字节 **3,206→3,598**；seed 42：114→38、460→156、**3,206→1,232** 字节。两种策略有明显的种子敏感与流量/计算费用取舍。
- **结论边界**：S16 额外知道合法 FOLD32 请求体、ACK/RESULT 响应形状和 P2 TLV 语法；策略在研究端预置，非活动 VM 学习，更不说明任意陌生服务无需反馈即可学习。

### S17 — 不告知家族的有限三协议参考求解器

- [S17 记录](./experiments/E001/DEV-S17-THREE-FAMILY.md)；[成功 CI #44304851025](https://github.com/iFrankStudio/OCEE/actions/runs/44304851025)：**77 项工作空间测试通过**，含新增两项关键三协议参考测试。
- 同一个 `ocee-calibration-solver` 模块以**无家族标签**的公开 HELLO 请求/响应作为输入，检查 P0 LengthXor、P1 EscapedDelimiter、P2 NestedTlv 三种**研究者已声明**的语法假说；唯一匹配时还原此实例协议字段/校验信息，再以 S16 的“有效请求→合法 ACK→合法 RESULT”策略找到隐藏 opcode。
- 三家族×六个开发种子的协议级测试、三家族×两个开发 seed 的真实 A→C→B 消息测试均成功。参考求解器收到的只有帧或通用 `Other`，从不看到 `Family`、`Protocol`、实际 B gas 或隐含结果；测试结束后独立审计 FOLD32 值、B gas 20、A 单 epoch 12,000 gas 配额和世界快照一致性。
- 旧 `crates/protocol-sim/tests/restricted_probe.rs` 虽有受限探针注释，却通过特权 `b_gas_charged>0` 判断目标是否成功，不符合信息受限口径，已由更严格的独立参考模块及其测试替代。**P0/P1 的旧证据应以 S17 为准。**
- **严格限制**：没有第四种协议语法生成、没有新原语发明、没有活动 VM 自学、更没有跨 episode 迁移；初始三协议模板、有效 FOLD32 工作负载、响应辨别规则全部 researcher-seeded。仍不足以满足 G1/G2。

### S18 — P1 活动 VM 的完整远端 FOLD32 闭环

- [S18 研究记录](./experiments/E001/DEV-S18-ACTIVE-P1.md)及[CI #44313014356](https://github.com/iFrankStudio/OCEE/actions/runs/44313014356)：81 项 workspace 测试通过。
- 同一个研究者安装的初始活动 VM 完成 HELLO 观察、P1 样例推导 XOR 键、112 操作码上界试探、真实 C 交互、发现 REMOTE_EVAL、提交公开三元素 FOLD32 工作项、重新发现 RESULT、验证响应与持久化 u32；seed 7/42/99 结果均通过外部独立算式核验，B 仅计费 20 gas。
- VM 新增通用 `ByteStuffEncode/Decode` 和 `PackU16Be`，操作计入 VM gas。禁用 BadBody 选择时不调用 B；破坏 RESULT 帧识别时 B 仍完成计算，但个体不存结果、不能误报成功。
- **先验与结论限制**：P1 字节布局、转义、XOR、诊断选择、固定工作项构造、RESULT 形状和有界搜索策略均由研究者直接写入初始程序；未产生新程序版本，不能证明陌生语法发明、P2 活动 VM 学习、跨 episode 迁移或 E001 M0。G1/G2 保持 NO-GO。


- **S18 最终全生命周期计量**：[CI #44313014356](https://github.com/iFrankStudio/OCEE/actions/runs/44313014356) 81 项 workspace 全通过；seed 7/42/99 的实际 SEND 数为 177/23/125，世界 tick 2,270/124/1,331，VM gas **29,149/3,779/20,385**，A 总 gas **31,446/4,998/22,318**，B 各 20。默认 A 12,000 gas/epoch，seed 7/99 跨越多个 epoch。与 S17 宿主参考算法没有计入其 CPU 的 A 动作账本不能直接对比；详见 [S18 成本口径](./experiments/E001/DEV-S18-ACTIVE-P1.md)。

### S19 — 活动 P1 VM 生成可执行工具、跨世界继承候选

- [详细实验与先验审计](./experiments/E001/DEV-S19-ACTIVE-TOOL.md)；[S19 首轮 CI #44330835759](https://github.com/iFrankStudio/OCEE/actions/runs/44330835759) 成功，工作空间 **84 passed / 0 failed**。
- 活动 VM 从 P1 HELLO 和 BadBody 反馈中发现 checksum key 与 REMOTE_EVAL opcode 后，用两次受计费的 `RewriteOp` 特化预置编码器，计算不可变内容哈希，保存版本；只有 `EXEC` 输出匹配独立构造的预期报文，才写入工具指针并实际调用生成 B 工作项，最终 FOLD32 结果经过研究端公式审计。三 seed 7/42/99 均通过。
- 有效类型、`EXEC` 返回 Ok 但分隔符错误的反例会拒绝注册与远端执行，不能把“可运行”误判成“有用工具”。
- 跨世界 P1 工具继承、新世界真实 C 的 `BadBody` 探针验证、不兼容回退及 CAS(0) 替换，均由活动 VM 实施。[最终 CI #44335557937](https://github.com/iFrankStudio/OCEE/actions/runs/44335557937) **87 项 workspace 全通过**；在第二 episode 同一 S19 初始程序的比较中，兼容继承 **A gas 6,046 vs 冷启动 6,669**、tick 129 vs 130；不兼容继承回退 **A gas 7,070 vs 冷启动 6,669**、tick 139 vs 130；两组都包含 62 gas 的导入，所有实际 B 工作计费 20 且取得正确结果。对照新世界使用不同的 world seed 和公开工作项；兼容组 P1 protocol seed 保持不变，不是跨协议泛化。已验证 S19 在特化产生不可变工具版本而尚未注册之时进行快照恢复，终态与不间断执行一致，B 仅计费一次。
- **限制**：工具完整算法与特化结构被研究者预置；S19 是受限的代码特化和实用能力证据，不是陌生协议语法自动发现、原创算法或 E001 M0。G1/G2 NO-GO。

### S20 — 活动 P1 VM 双可执行工具的复用、局部回退和累计资源收益（已验证）

- [S20 完整工程研究报告](./experiments/E001/DEV-S20-DUAL-TOOL.md)，[最终 CI #44340938612](https://github.com/iFrankStudio/OCEE/actions/runs/44340938612) **93 项 workspace 全通过 / 0 失败**。与 S19 只有 REMOTE_EVAL 工具不同，活动 VM 经合法 C RESULT 响应发现隐藏 RESULT opcode 后，自己 `RewriteOp` 研究者预置编码模板，算哈希、写不可变程序、孤立 EXEC 校验并注册第二工具。
- 同一初始 S20 VM、全新 world seed128 / 相同 P1 protocol seed42 的对照：**冷启动 8,354 A gas / 136 tick**，仅继承 REMOTE_EVAL **7,731 / 135**（导入62 gas），两个工具都继承 **3,851 / 33**（导入124 gas）。双工具对冷启动节省 53.90% A gas，且每个成功世界 B 仅计费20。
- 失效旧经验的分离对照：两个不兼容工具继承时会实际验证并回退，A gas **9,144**、tick154，对冷启动增加790 gas；只有 RESULT 工具失效、REMOTE_EVAL 仍兼容时仅重新学习 RESULT、CAS(0) 替换其引用，A gas **8,120**，其它工具不受牵连。不能只选择正面节省结果。
- 三个 world seed 42→128→129 的新世界，**protocol seed 固定42**；跨 episode 只按受计费 WRITE 搬运两个不可变程序及引用，每轮不同公开工作项，所有 B 各计费20。首轮学习加两次继承累计 **16,056 A gas**，全冷启动 **25,064**，节省 **9,008 gas (35.94%)**，两次导入各124 gas均计入。
- 在生成的 RESULT 程序**已 WRITE 但 registry 尚未 WRITE**的状态快照重启，继续执行终态与未中断轨迹完全相等、B 未重复计费。
- **严守限制**：两份编码器的算法、P1 字节规则、修改点、任务形状、扫描/响应选择和成本优化策略均由研究者初始程序赋予，**仅证明同一 P1 实例的有限工具积累，不证明 P2/陌生语法、自发新原语或开放式演化**。S20 程序初始哈希由 CI manifest 输出，当前 **G0 GO，G1/G2 NO-GO**。

### S21 — P2 HELLO 字段推断与新请求实际使用（开发集）

- [S21 报告](./experiments/E001/DEV-S21-P2-TLV.md)，[CI #44355391031](https://github.com/iFrankStudio/OCEE/actions/runs/44355391031) **99 项工作空间全通过**。VM 增加四个通用 TLV 字节原语，均显式按字节成本计费；它们不内置 P2 私有 tag、物理顺序或 opcode。
- S21A 一个受限活动 VM 实际从公开 HELLO 的 request/response 对推断五角色 tag 与五字段物理顺序；六 P2 seed 7/42/99/272/335/1001 均匹配研究端事后独立验算。5 tick，A gas 3404–3406，B0；把 response 替换为同一 request 消融后不得伪造完整角色。
- S21B 初始活动程序使用自己的上述结果及公开 HELLO opcode 重新组装 request ID73、发送真实 C 并验证回帧；seed7/42/99/272 A gas4230/4231/4231/4231，各12 tick、SEND1、B0，研究端独立解码确认是真正的新合法 HELLO，不是公开样例重放。响应 body 判据故意错误时即使 C 已接受也拒绝保存。
- **研究限制**：五字段 TLV 的框架、固定 HELLO 行为、判别/验证规则及新通用 TLV 原语仍由研究者预置；S21A/B 是不同初始程序，既非 S20 P1 的跨家族迁移，也没有发明完整未知语法、发现 P2 私有 REMOTE_EVAL/RESULT 或完成 B 计算。G1/G2 NO-GO。

### S22 — 同一活动 P2 VM 的隐藏操作码搜索与 B 完整闭环（定向验证成功）

- [S22 完整记录](./experiments/E001/DEV-S22-P2-B-CYCLE.md)、[定向 CI #44440713315](https://github.com/iFrankStudio/OCEE/actions/runs/44440713315)：**3 项因果核心测试通过，0失败**；[全 workspace CI #44440432805](https://github.com/iFrankStudio/OCEE/actions/runs/44440432805) **104 项通过，0失败**，包括既有 S0–S21 核心回归。
- 首次在**同一个活动 VM**内串联 P2 公开 HELLO 字段 tag/order 推断、合法 frame 构建、112候选有界 REMOTE_EVAL 扫描、C 的 BadBody 诊断选择、从公开 job0 提交真实 B FOLD32 3*u32 任务、合法 ACK 校验、RESULT opcode 有界扫描及真实 u32 回收。私有协议参数和 B 账本仅供**研究端事后核验**，不是个体输入。
- seed7/42/99：世界 tick **2071/1047/1037**、A gas **25,076/12,953/12,599**、其中 VM gas **20,601/8,870/8,523**、SEND **96/40/39**、B gas均20。真实工作正文和返回值与独立 FOLD32 校验一致。
- 负对照：取消 BadBody 选择时 B0；错误 RESULT 预期长度时 B20但不保存假结果。注意它们仅约束研究者预置策略的因果证据。**完整 P2 TLV 五字段和 XOR、诊断及工作语义依然由研究者播种；不证明跨语法单个体演化或原创算法。**
- S21 的恶意内层 TLV 可能引发 InvalidCode 而非优雅故障，S22 尚未消除这一已确认缺口；不能把故障封闭误称为主动自愈。

### S23A — P2 可执行 REMOTE_EVAL 工具特化、验收及继承（定向验证通过）

- [S23A 实验与工程记录](./experiments/E001/DEV-S23-P2-EXECUTABLE-TRANSFER.md)、[三核心对照 CI #44455627472](https://github.com/iFrankStudio/OCEE/actions/runs/44455627472)：**3 passed / 0 failed**。完整 workspace CI [#44455627462](https://github.com/iFrankStudio/OCEE/actions/runs/44455627462) 的 `kernel` job 已结束为 **cancelled**，并非 success；因此 S23A 目前只有三项定向对照成功的 CI 证据，不能宣称该提交完整 workspace 回归通过。
- 同一个活动 VM 用 S21A 公开 HELLO 得到 P2 私有 role/order，再用 S22 的合法 C BadBody 得到 REMOTE_EVAL opcode，在 VM 内完成研究者预置的 TLV 编码器两处 RewriteOp、内容寻址、隔离 EXEC、C 功能性验收、CAS 注册。真实 B FOLD32 工作报文**必须由注册程序 EXEC 生成**，并由研究端独立验证返回值。
- 同 S23A 初始程序、target world128/protocol42：**冷 19,712 A gas/1132 tick**、兼容继承 **14,452/1057**（导入102、节省26.7%）、失效继承并 CAS 重学 **20,319/1142**（导入102、负迁移+3.1%），B 均20。seed99 的独立冷启动 19,277 gas/1127 tick 也通过。
- 先遇到128基本块和64KiB容量不足：没有保留任何放宽上限，改用研究者的五次有界角色识别共享图；共享反馈环由一次性 bounded 阶段约束。seed99 还暴露合法 RECEIVE=Empty 导致 InvalidCode，现由最多128次计费等待/重试处理，只有返回 Ok 才解析 payload。**不应将这些研究者故障排除误报成个体自行修复**。
- 先验 P2 格式、112候选、BadBody判据、FOLD32任务和完整可执行模板均由研究者预置；只证明已知格式下实例特化与迁移节约，RESULT 尚需冷搜索，P1→P2 同初始个体迁移尚无证据。G1/G2 NO-GO。

### S24 — 把异常 P2 回复转为可处理失败，并从真实 RESULT 补救任务

- [S24 完整实验记录](./experiments/E001/DEV-S24-RECOVERABLE-TLV.md)，[定向 CI #44502050065](https://github.com/iFrankStudio/OCEE/actions/runs/44502050065) **7项通过**。VM 加入研究者编写的通用总函数 `TlvMatchesOrder`，逐字节计费、输入 tag/order 为个体观察数据。S21B 回复路径在无保护 `TlvLookup/TlvReorder` 前检查，S22/S23 共用 ACK/RESULT 解析也以布尔判断拒绝不匹配内层字段。公开 C 回复额外合法内层 TLV 从过去的 `InvalidCode` 变为个体返回 `U32(0)`，不保存伪成功。
- S24B 是单独研究者设计的有界恢复程序：C 实际处理 B 工作且已扣费20，在 ACK 交付前被研究端注入额外 TLV；个体不承认损坏 ACK，改由合法 112候选 RESULT 探测取回独立 FOLD32 确认的答案。seed42：旧版1047tick、12,912 A gas；新版 [CI #44503380118](https://github.com/iFrankStudio/OCEE/actions/runs/44503380118) **1122tick、18,845 A gas（VM 14,663）、B20**，没有重复 B 工作。安全拒绝版本在同一注入后72tick返回失败、B也20。
- **限定**：修复与恢复策略均为研究者更新的初始程序和通用 VM 原语，不是活动个体的自我编程发现；外层无法解析的畸形 TLV、任意损坏字段、其它未观察故障仍不保证可恢复。S24 当前定向通过、全 workspace [#44502050056](https://github.com/iFrankStudio/OCEE/actions/runs/44502050056) 尚须核验。G1/G2 NO-GO。

## 已发现且修复的关键问题

- C/B 服务作业表初版使用 `BTreeMap<(actor, request_id), job>`，无法直接序列化为 JSON 对象键，导致世界快照计算失败；已改为稳定编码的字符串键，并通过世界快照重放验证。
- S2 首次结果查询使用了含计算负载的 REMOTE_EVAL 帧而不是 S1 的空负载模板，导致即便猜对 RESULT 操作码仍返回 `BadBody`；通过校正跨块传递的帧来源修复，重放测试通过。
- `EXEC` 的隔离读必须计入 gas；目前只允许本地授权观察与只读（含公开 HELLO 样例），禁止向 C/B 发送消息或修改对象。
- VM 循环可能跨多个 gas slice；循环计数必须属于可持久化 continuation，而不是非持久的宿主临时变量。

## G0/G1/G2 检查

| 闸门 | 状态 | 剩余必要工作 |
|---|---|---|
| G0 工程开工 | GO | 当前已在实施 |
| G1 允许开发/校准运行 | **NO-GO** | 具有非模板结构创新的初始 S 程序+冻结 hash、独立求解器更广泛协议/任务/预算校准、工作项/资源收益校准、必要持久化和权限审查、对照组实现 |
| G2 允许正式确认性 E001 | **NO-GO** | G1 通过后冻结 seed、统计功效/门槛及代码，预注册并确保开发/正式集隔离 |

**下一步推荐**：S14 已完成有限任务的迁移闭环；以 S18 活动 P1 完整闭环为基础，下一步在 S22 已通过真实 B 计算闭环的基础上，让 P2 实例级操作码与构造能力变为可复验/可继承的可执行工具，比较跨 episode 复用收益与负迁移成本，再检验一个初始程序的 P1/P2 跨家族选择、TLV 假说失效的优雅处理和更广独立校准，并构建正式 C/D 状态重置和对照；继续研究非模板结构创新，再推进正式工作项/统计和 A/B/C/D 四组对照。对完整 ISA、SQLite 与权限模型采用进入 G1 前的必要性审查，避免无关扩张。保持所有结论与来源证据可追溯。

以上是**工程验收记录，不是数字生命、智能或开放式演化的证据**。


### 2026-11-19 UTC — S23A 全量 CI 超时诊断与恢复待验

- [原始全量运行 #44455627462](https://github.com/iFrankStudio/OCEE/actions/runs/44455627462) 的 `kernel` job 第一次为 cancelled；定向重跑（attempt 2）在 **2026-11-19 07:12:37–07:32:53 UTC** 再次 cancelled，耗时约20分钟，恰好触及 `.github/workflows/rust-kernel.yml` 原有 `timeout-minutes: 20`。不能把取消当成测试通过或 Rust 断言失败；也不能排除有测试卡住，须看后续完整日志与运行结果。
- [工作流调整 ad4e515](https://github.com/iFrankStudio/OCEE/commit/ad4e515fc1b7abf7db4c7ff2acd57f0f308bfe6d) 将全量 job 的有限超时由20提升到35分钟，保留相同 `cargo test --workspace -- --show-output`、S8成本与manifest命令，不增加测试、不放宽个体 gas/存储/VM 上限。此变更触发新的工作流；**尚未取得该提交的完成 CI**。若35分钟仍未通过，必须定位慢测/挂起并优化，而非继续盲目延长。
- [S23定向 #44455627472](https://github.com/iFrankStudio/OCEE/actions/runs/44455627472) 三项通过仍有效，但不代替完整回归。S21内层TLV `InvalidCode` 未解决；这次是研究者 CI 设施维护，不是个体自愈。G1/G2仍 NO-GO。


### 2026-11-19 UTC — S23A 全量回归恢复：35 分钟上限内成功

- [工作流运行 #44485672595](https://github.com/iFrankStudio/OCEE/actions/runs/44485672595) 对应代码/工作流提交 [`ad4e515`](https://github.com/iFrankStudio/OCEE/commit/ad4e515fc1b7abf7db4c7ff2acd57f0f308bfe6d)，GitHub Actions 已报告 `completed / success`。`kernel` job 的 workspace 测试、S8 生命周期成本报告和开发个体 manifest 三个步骤均为 `success`；此前两次 20 分钟取消已由这次完整通过取代为最新全量回归证据。运行于 2026-11-19 08:12–08:37 UTC 完成，未触及 35 分钟上限。
- 本次通过仅证明当前工作空间测试和附加命令在该 CI 环境下完成；不证明测试外所有协议输入安全，也不证明 S21 内层 TLV `InvalidCode` 已修复。S23A 工具继承仍属人工预置生成/验证策略的有限开发集能力；未开始正式 E001，G1/G2 仍 **NO-GO**。
- 后续聚焦 S21 的异常内层 TLV 可恢复性：先将错误反馈与 VM 终止区分，再在同一活动个体内测试受限通用图编辑、独立自测、CAS 采用与跨 episode 复用。不能把研究者实现的错误处理算作个体自主发现修复。避免为单一新增测试重复运行全量耗时 CI。

### 2026-11-19 — S24 共用 P2 回复路径回归诊断

- [S24 有界恢复研究记录](./experiments/E001/DEV-S24-RECOVERABLE-TLV.md)。S21 和真实 B ACK 注入的 focused [CI #44502050065](https://github.com/iFrankStudio/OCEE/actions/runs/44502050065) 7项通过。
- [S22 定向 #44501847870](https://github.com/iFrankStudio/OCEE/actions/runs/44501847870) **failure**：BadBody 屏蔽组 seed42 1028 tick 合法 `RECEIVE=Empty` 未处理而 `InvalidCode`。正常三 seed 及错误 RESULT 长度对照通过不能掩盖该失败。
- [6abfdfa](https://github.com/iFrankStudio/OCEE/commit/6abfdfaad6634b8be2abb9244c4e9fd339ab8854) 已把 S23 现有有界空接收处理搬到 S22 基类，并让 S23 复用已有等价逻辑，避免重复基本块。S22/S23/S24 后继定向及全 workspace CI 待成功结果，**G1/G2 仍 NO-GO**。


### 2026-11-19 S24 历史全量回归：seed99 正向失败（后继已复现并修复）

- 本次复核 GitHub Actions **S22 [#44503380110](https://github.com/iFrankStudio/OCEE/actions/runs/44503380110) failure** 与完整 workspace **[#44503380220](https://github.com/iFrankStudio/OCEE/actions/runs/44503380220) failure**，两者共同指向 `crates/organism-seeds/tests/s22_p2_remote.rs:54` 的 `active_vm_p2_learns_both_hidden_operations_and_executes_real_b_fold32`，**seed99 返回 `Some(U32(0))`，预期 `Some(U32(1))`**。seed7/42 的同一正向用例已成功，阴性组两项通过；这不是单纯超时/取消，也不能宣称 S24 整套全量通过。
- 同一修订的 **S23 [#44503380158](https://github.com/iFrankStudio/OCEE/actions/runs/44503380158) success** 与 **S24 [#44503380118](https://github.com/iFrankStudio/OCEE/actions/runs/44503380118) success** 不能替代失败的 S22/全量。S24B seed42 资源计量已经变为 1122 tick、A 18,845 gas、VM 14,663 gas、B 20；旧成本不可沿用。
- 代码复核：S22 `s22_guard_receive` 对 `Empty` 使用 `RepeatBounded(128)`，耗尽后直接 `failed`；但现有测试仅检查最终返回值，**未记录 seed99 终止时的具体分支、是否重试耗尽、最后反馈或 C 服务事件**。因此目前只能把 128 次等待预算、消息延迟与资源调度视为待验证假说，不能写成已确认根因。
- **下一执行检查点**：先在 S22 seed99 单独增加最小诊断（终态 continuation/最近 RECEIVE 状态/实际发送及服务事件；不读私有答案作为控制输入），重现失败并确定因果；若确认重试耗尽，优先修正有界等待的语义/预算而非无限循环或盲目扩大限额。随后只运行 S22 定向 CI，成功后再考虑全量 CI。保留确定性、配额、错误拒绝和负面组；此处属于研究者工程修复，不是个体自主修复。**G1/G2 NO-GO**。


### 2026-11-19 S24 后继核验：共享预算耗尽已复现，分阶段真实高延迟重试通过

- [完整审计](./experiments/E001/DEV-S24-RECOVERABLE-TLV.md) 第6节；[历史重放定向 #44517653790](https://github.com/iFrankStudio/OCEE/actions/runs/44517653790) **2项通过**，同一 seed99 旧的全生命周期共享128次 Empty 限额**确实耗尽**：终态 U32(0)，tick1284、A14,197、B20。
- [研究者修复提交 0e8458c](https://github.com/iFrankStudio/OCEE/commit/0e8458c288802eb919d281c39dff9508f8b2a1f5) 把 S22 的空接收细分为远端操作码扫描、真实 B ACK、RESULT 三个不同 RepeatBounded（各 cap256，保持原有 VM 上限、真实成本和持久计数）。新 seed99 在默认网络延迟下 U32(1)、A18,529、B20，但三计数均为0，不能仅凭此例证明真正经历了高负载 Empty。
- 增加模拟消息延迟并保留全部 P2 私有参数及任务语义不变：delay4 的真实 Empty 为 **6+2+70=78**、A20,309、tick1207、B20、任务仍完成；最初 [#44518525140](https://github.com/iFrankStudio/OCEE/actions/runs/44518525140) 因测试设定的超过128压力门槛而失败，不能叫作计算失败。delay6 的真实 Empty 为 **12+4+140=156**，且每段≤256、最终 U32(1)、tick1313、A22,087、B20，结果匹配独立 FOLD32；[定向 CI #44519428630](https://github.com/iFrankStudio/OCEE/actions/runs/44519428630) **3项通过**。
- [同代码的 S22 定向 #44516301596](https://github.com/iFrankStudio/OCEE/actions/runs/44516301596) **3通过**、[S23 #44516301537](https://github.com/iFrankStudio/OCEE/actions/runs/44516301537) **3通过**、[S24 ACK 损坏补救 #44516301516](https://github.com/iFrankStudio/OCEE/actions/runs/44516301516) **7通过**。新 S22 seed7/42/99 A 分别39564/18986/18529，B 各20；S24B 受损 ACK 补救 A18,952、tick1122、B20。以上为新程序计费，不得沿用旧哈希的 gas。
- **完整 workspace [#44519428660](https://github.com/iFrankStudio/OCEE/actions/runs/44519428660) 已通过113项**，可用作 S24 基线；S25 新增快照试验仍需以自己的全量 CI 核验。该修复由研究者显式重构 VM 初始程序图，未证明赛博个体主动产生容错策略，更没有跨家族语法或开放式演化。G1/G2 继续 NO-GO。

### 2026-11-19 S25 — 真实高延迟状态恢复的完整审计一致性

- [S25 研究记录](./experiments/E001/DEV-S25-MID-RECEIVE-RESTART.md)、[定向 CI #44524418058](https://github.com/iFrankStudio/OCEE/actions/runs/44524418058) **1项通过/0失败**。S24 原 P2 活动程序不变，world/protocol seed99、C delay6；在 step1225、A gas18,978、B gas20、三段实际 Empty=[12,4,113]（共129），尚未保存 RESULT 时，测试主持者真实 JSON snapshot/reload。
- 恢复后的 VM continuation 持有相同有界计数、pending transport、审计链和资源状态，两条世界继续执行均在 step1313、A gas22,087、B gas20、Empty=[12,4,140]（共156）完成真实 FOLD32。逐条 `World.audit` 完全相等、最终 state hash 相同 `c0f2dca98cbe5825bdd8678db7c07af3a4641e547f2a398b52c8d6de4af02977`。不是只做完成后的快照 round-trip。
- S25 只加入一个紧凑因果测试与独立定向 CI，不增加个体特权、VM 指令、重试预算或者新的宿主修复规则。**这是工程的确定性续执行证据，并非个体主动备份、重新发明控制流或开放式演化**。全量 [#44524418089](https://github.com/iFrankStudio/OCEE/actions/runs/44524418089) 尚待结论。G0 GO、G1/G2 NO-GO。


### 2026-11-19 — S26A 跨家族继承工具的可恢复拒绝（CI 尚待原生验收）

- [S26 配对研究契约](./experiments/E001/DEV-S26-CROSS-FAMILY-PAIRED.md)与 [开发集配对分析工具](./experiments/E001/s26_paired_audit.py) 已提交；没有任何来自原生实验的 C/D CSV，不得声称 H26 已验证或 G1 通过。
- [S26A 原生负迁移测试](./crates/organism-seeds/tests/s26_foreign_family.rs) + [定向工作流](./.github/workflows/s26-foreign-family.yml) 已提交。测试由实际 P1 S19 活动 VM 生成可执行编码器，经按真实 WRITE 计费的跨 episode 导入转交 P2 S23 VM，比较同 world/protocol/job 的冷启动对照。要求异构 P1 报文先被 P2 个体拒绝而非 InvalidCode，再由它自行通过当前公开 P2 反馈重新特化/注册编码器、EXEC 发出真实 B/FOLD32 请求、结果独立核验且 B 只扣20 gas；失败继承成本必须计入。
- [S23 继承路径修订](./crates/organism-seeds/src/s23_p2_transfer.rs)：在已有 EXEC 的合法 Bytes 输出被当作 P2 结构之前，加入**个体内部的受计费检查**：至少3字节、HELLO 已学 marker、单个外层 tag=0x10 TLV 结构、内层五字段实际 tag 顺序与已学角色吻合。不满足就走原有冷学习分支，禁止直接进入可能抛 InvalidCode 的 TlvDecode / TlvLookup。这是研究者编写的受限容错图，不是活动个体自发修复。
- 本节记录的是**已提交的实验与可证伪断言，不是测试已经通过**。需先看 S26 定向 GitHub Actions 的 completed/success 与实际 A/B/导入 gas、工具版本，再记录结论；完整 workspace 在新修改上同样尚待核验。
- 即使 S26A 通过，也仅说明拒绝跨家族直接复用与可恢复重学；**C/D 均非同一冻结跨家族通用初始 VM**，不满足 E001 的跨家族正迁移/原语发现判据。下一步仍需统一初始个体+真实配对探索/记忆消融。G0 GO、G1/G2 NO-GO。


### 2026-11-19 — S25 全量 #44524418089 最终取消（独立核验）

- 通过 GitHub Actions 原生 jobs 查询确认 [S25 定向 #44524418058](https://github.com/iFrankStudio/OCEE/actions/runs/44524418058) 的 `mid-receive` job 为 `completed/success`，但 [S25 全 workspace #44524418089](https://github.com/iFrankStudio/OCEE/actions/runs/44524418089) 的 `kernel` job 为 `completed/cancelled`。**不能把 S25 的全量标为通过**；已确认的最近一次全量成功仍是 S24 的 [#44519428660](https://github.com/iFrankStudio/OCEE/actions/runs/44519428660)，113 项通过。
- S26A 针对性 Rust/CI 已提交，但尚无经原生日志核验的 completed/success；不要在新 workflow 出结果前报告 S26A 的 gas、CAS 版本、B 结果为实测成功。G1/G2 仍 NO-GO。

### 2026-11-19 — S26A 负迁移定向及后继全量 CI 已完成（独立复核）

- [S26A 定向运行 #44535284980](https://github.com/iFrankStudio/OCEE/actions/runs/44535284980)，head `6a987303`，GitHub Actions `foreign-family` job 和实际 Rust 测试步骤均为 **completed/success**。测试断言覆盖：真实 P1 S19 VM 产生并注册可执行工具；P2 S23 VM 对跨家族导入执行受计费验证，拒绝外来结构、重新特化并 CAS 替换；两个 P2 世界均成功，B 各扣 20 gas、结果独立核验，导入组 A gas 严格高于冷启动组。此处的成本差方向来自测试断言，**尚未从日志提取精确 gas 数字**，不得编造。
- [后继全量工作空间 #44535255224](https://github.com/iFrankStudio/OCEE/actions/runs/44535255224)，head `d1df6c58`，`kernel` job **completed/success**；`Run world and VM invariant tests`、S8 生命周期成本报告和个体 manifest 三个步骤均为 **success**。此结果是 S26A 后继代码的全量回归证据，不会倒填此前被取消的 S25 原运行。
- **能力来源与负面结论**：P1 与 P2 的初始活动程序不同，且外来帧结构检查由研究者编写；因此本结果仅证明跨家族错误工具可安全拒绝并重学，属于**负迁移成本**而非结构正迁移、通用协议发现或自主发明。S26 C/D 配对 CSV 及统一初始 VM 尚未完成，正式 E001 未启动，**G0 GO / G1、G2 NO-GO**。
- **下一执行检查点**：以一个冻结的跨 P1/P2 初始 VM 哈希构建原生 C/D 配对；仅迁移个体产生的对象/工具，逐条计费并记录世界审计；先做一对真实跨家族运行和工具移除反事实，再生成开发集 CSV 交由 `s26_paired_audit.py` 检查。不要复用两套不同的研究者程序冒充同源对照。

### 2026-11-19 — S26A 实测负迁移成本与 S26B 同源 VM 判别

- [S26A 定向 CI #44535284980](https://github.com/iFrankStudio/OCEE/actions/runs/44535284980) 的原生作业日志已独立复核：P1→P2 seed42、target world128；冷启动 **A 19,843 gas / 1,132 tick**，带 P1 错误工具继承 **A 20,045 gas / 1,137 tick**（其中写入导入成本 **62 gas**），因此可观察到 **+202 gas（约+1.02%）负迁移成本**。两组真实 B 各20 gas、独立 FOLD32 正确、P2 重新学习并 CAS 替换。此前“暂无精确 gas”由本条日志核验数字补充。不是跨语法正迁移。
- [S26B 同源 VM 提交 aeab984](https://github.com/iFrankStudio/OCEE/commit/aeab984616035b1fff44c7f271f5994144cb958d) 和 [定向 CI #44548600143](https://github.com/iFrankStudio/OCEE/actions/runs/44548600143)：**1 项核心 Rust 测试通过**。同一冻结的活动初始程序哈希在 P0/P1/P2×开发 seed7/42/99 共9个世界运行，使用实际公开 HELLO request/response，不向个体传 Family、私有 generator、隐藏 opcode、B 账本。P0 对照正常返回未分类，无 P1/P2 伪结果；P1 识别公开的 0x7e 前后分隔符、P2 识别通用 outer TLV tag0x10，并经世界 WRITE 持久记录。P0 A gas **456/456/457**、tick4；P1 **487/487/486**、tick5；P2 **533/533/533**、tick5；B 均0。相同哈希断言与快照重放成功。
- S26B 是**研究者播种的两种已知字节格式假说的有限选择**。它解决“必须有单个可测试初始程序入口”的前置障碍，但没有自主发明解析器、没有真正 C→B 工作，不能当成跨家族 H3 或开放式演化。S26B 全 workspace [#44548600138](https://github.com/iFrankStudio/OCEE/actions/runs/44548600138) 在本条撰写时仍须核验最终结论。
- [S26C 活动程序包提交 8634dc4](https://github.com/iFrankStudio/OCEE/commit/8634dc4b55261d9cb8b924aa3e1df2562988e6d5)：同样的封存 router + P1 S19/P2 S23 **两份相同预先安装的程序库**，让 root 根据公开 HELLO 自行受计费 WRITE(/self/active) 切换到专用程序，必须有真实 B FOLD32 与逐条审计。专用模块全部为研究者提供而且在两世界等额安装；**即使测试通过也只是受先验限定的自主选择**。定向 [#44549321018](https://github.com/iFrankStudio/OCEE/actions/runs/44549321018) 的结果尚待验收。
- G0 工程 GO；G1/G2 仍 **NO-GO**。后续关键是固定整个初始程序包并让它自行生成、验证、继承跨家族可用的工具，而非仅选择两份人工专家。


### 2026-11-19 — S26C 单一初始程序包的真实远端计算（已核验定向 CI）

- [定向 Rust CI #44549321018](https://github.com/iFrankStudio/OCEE/actions/runs/44549321018) **completed/success，1项核心测试通过/0失败**；对应 [提交 8634dc4](https://github.com/iFrankStudio/OCEE/commit/8634dc4b55261d9cb8b924aa3e1df2562988e6d5)，测试源码 [s26_dispatch.rs](./crates/organism-seeds/tests/s26_dispatch.rs)。**P1 和 P2 世界使用完全相同的 root hash** `03a2ca2e477b6d9985fd16e0ed457e67ceeafbb5c1d9789623821a343d7e3a46`，两种专用程序 S19 P1 / S23 P2 在两世界都同时预置并按实际 WRITE 计费，没有宿主按家族安装不同程序。root 使用公开 HELLO 结构证据判别，写入 /self/discoveries/framing-family，再在**活动 VM 内一次受计费 CAS WRITE(/self/active)**切换到匹配专用程序；世界内核清除旧 continuation，后续 tick 自主执行被选程序。
- 原生日志：world/protocol seed42，P1 **A 9,762 gas、133 ticks、B 20 gas**；P2 **A 21,833 gas、1,163 ticks、B 20 gas**。这两组 A gas **包含两份研究者预安装专用程序以及 router 的初始写入成本**，不可与未计安装成本的其它历史实验直接比较。二者真实 C→B 计算、独立 FOLD32、世界快照状态一致性、单次 VM 自主 active 切换、P1/P2 结果持久化的因果断言均通过。
- **先验来源审计**：这**不是一份单体新算法**，而是研究者先验的公共入口加两份已知协议专家的同源包，选择逻辑也由研究者给出；没有从 P1 创造出跨 P2 有用的新协议工具，没有 C/D 继承收益，更无开放式演化。其意义仅为排除后续“测试宿主按已知世界手动选专家”的混杂因子，建立同源执行与等初始能力的实验地基。
- S26B 独立定向 #44548600143 已通过，但其全量 #44548600138 随更新 main 的新代码推送成为 **cancelled**，不得写为通过；S26C 后继完整工作空间 [#44549320903](https://github.com/iFrankStudio/OCEE/actions/runs/44549320903) **尚待完成**。G0 GO；G1/G2 **NO-GO**。
- **下一真正决定性步骤**：在上述同源初始程序包基础上，实施由个体自己生成并在跨家族世界**功能性复用**的非字节模板工具（如经真实交互选择的通用校验/结果解析子程序），匹配 C/D pair 的 budget、seed、权限与初始化，移除继承工具作结构性反事实。若只是运行预置 P1/P2 专用库，禁止提升 G1 或 H3 结论。


### 2026-11-19 — S27 同源初始程序包的真实配对对照：Rust 通过，分析流水线修复中

- [S26C 完整 workspace #44549320903](https://github.com/iFrankStudio/OCEE/actions/runs/44549320903) 已经独立核验 **completed/success**：Rust workspace invariant、S8 cost、manifest 三步骤均通过，不能继续写成待核验。
- [S27 配对测试](./crates/organism-seeds/tests/s27_transfer_pairs.rs) 的第一轮 [Actions #44558035166](https://github.com/iFrankStudio/OCEE/actions/runs/44558035166)：**原生 Rust 1项通过，0失败**，用时274.17秒；但独立 Python CSV 审计步骤发生 FileNotFoundError，导致**整个 workflow 为 failure**。根因是 cargo test 默认在 crate 目录执行，而测试用相对路径 target/s27_cross_family.csv 保存指标、Python 却从仓库根目录读取。已通过 [CI 修复 022df157](https://github.com/iFrankStudio/OCEE/commit/022df15700171d473c003ef0e6d3554fc63f0ae3) 使用 `github.workspace` 绝对路径；**修复后的完整流水线尚待证据**，不能写成已通过。
- Rust 原生执行数据：**完全相同 root + S19(P1)/S23(P2) 初始程序包**，source world/protocol seed42，target P2 world128/protocol42。Target 无继承 C：**A 21,833 gas / 1,163 tick**。跨家族 D 携带 P1 自己生成的工具并保留 p1 原生命名：**A 21,895 gas / 1,163 tick**（真实导入 **62 gas**，额外 **+62**；P1 donor 工具**未被实际 EXEC**，不能声称 H3）。同家族 P2→P2 导入 D：**A 16,671 gas / 1,087 tick**（导入 **102 gas**），相对冷启动节省 **5,162 gas / 23.64%**；donor 工具真实 EXEC，冷启动有候选发现/测试，两者 B 各20。所有 source 和 target 真实 B 计算通过独立 FOLD32 验算与状态快照检查。
- 原生日志中用于 Python 跨家族审计的成对 C/D 行：source=P1、target=P2、两组成功1，A=21833/21895、B=20/20、发送字节1162/1162、ticks=1163/1163，导入工具 causal_tool_used 0/0，目标世界初始 state hash 相同。不得把同家族正对照误填进跨家族 H3 统计。
- **科学限制**：数据只来自匹配的一组开发 seed，个体能复用的是研究者提供的完整 P2 编码模板经过实例特化，不是自创新跨家族算法；P1→P2 工具未被消费，说明当前机制对该知识没有跨协议迁移效用。G0 GO、G1/G2 **NO-GO**；下一个真正研究点是 VM 内可生成、类型化、两家族都可消费的**协议无关功能子程序**，且要有移除工具的功能/资源反事实。避免继续用只共享文件、未调用的工具宣称迁移。


### 2026-11-19 — S27 全部定向流水线成功、配对 CSV 已归档

- 修复 CSV 工作目录问题后的 [S27 最终 CI #44559169797](https://github.com/iFrankStudio/OCEE/actions/runs/44559169797)：GitHub Actions `paired-evidence` job **completed/success**；Rust 真世界配对测试、独立 Python 配对审计、CSV artifact 上传三个步骤全部成功。原始 CSV [工件 14548366400](https://github.com/iFrankStudio/OCEE/actions/runs/44559169797/artifacts/14548366400) 已保存（GitHub 压缩 artifact SHA256 `cb7a32babcdd83e58b556e402b277795b4c9327696b99d8f4d4111b73c568369`）。具体方法和首轮失败的透明记录见 [S27 报告](./experiments/E001/DEV-S27-PAIRED-TRANSFER.md)。
- 独立 Python 输出：**P1→P2 跨家族，1个真实配对 seed**，C/D 成功各1，D−C 成功差0，声称 causal_tool_used=0，A gas 差 **+62**、B gas差0、发送字节差0、tick差0。与 Rust 原生事件审计所见的 P1 donor 工具未 EXEC 一致。S27 同家族 P2→P2 兼容继承节省 **5,162 A gas / 23.64%**，但 **不是**跨家族 H3。
- S27 新增核心测试是有效配对，不足以通过 G1：家族专家先验、未验证任何跨家族功能复用、未形成非模板结构创新，也没有正式预注册；G0 GO、**G1/G2 NO-GO**。后续应在功能性可迁移子程序上继续，而非只延长重复历史 CI。


### S27 审计回执修订的全量 CI 验收（2026-11-19）

- `main` 提交 [`43fc708`](https://github.com/iFrankStudio/OCEE/commit/43fc70805e530f44bc24a6f6e9400a6fbbae09ac) 将 `s27_transfer_pairs.rs` 的 `exec_seen()` 从仅见到 `Action::Exec` 收紧为还需同条世界审计 `result.code == Code::Ok`。
- 对应 [完整工作空间 Actions #44573987351](https://github.com/iFrankStudio/OCEE/actions/runs/44573987351) 已于 **2026-11-19 17:41 UTC completed/success**：`Run world and VM invariant tests`、`Report S8 lifecycle costs under original A quota`、`Print auditable development organism manifests` 均成功。该次是全量工作流，不应冒称单独重新运行 S27 的 CSV/独立 Python 审计；此前 S27 独立证据仍以 [#44559169797](https://github.com/iFrankStudio/OCEE/actions/runs/44559169797) 为准。
- **证据语义**：`causal_tool_used` CSV 字段当前至多证明成功的 `EXEC` 事件被审计观察到，**不证明工具对最终成功必要**。同家族节省 gas 与冷启动对照支持功能复用，但尚需严格的“保持其他导入/资源轨迹一致，仅移除或禁用工具”的反事实，才可强化因果归因。跨家族 P1→P2 的工具成功 EXEC 为 0，成本 +62 A gas，维持负迁移结论。
- 下一小实验优先在现有 S27 原生运行器增加真正同源、同计费导入、仅工具可用性不同的反事实（保留原始审计/失败行），避免引入新研究者专家模板；随后才研究跨家族可消费的通用子程序。**E001 未预注册，G1/G2 NO-GO**。


### S27B 2026-11-20 follow-up: sham schedule confound (development only)

- [S27 paired CI #44652186006](https://github.com/iFrankStudio/OCEE/actions/runs/44652186006) **FAILED** after 343.32 s: `paid but invisible tool must not improve runtime` expected sham A gas = cold A gas + import gas, but observed **21,839 vs 21,935** (difference **−96 gas**). This is a *new negative result*, not evidence of autonomous tool innovation.
- The sham tool is imported with the same P2 bytes/hash and equal WRITE import cost as same-family reuse but registered under an unconsumed name. A cold candidate object is still required. However, two extra import WRITE ticks alter scheduling/epoch boundaries and may alter resource waits. The exact cold+import gas/tick identity is **not justified** by matching candidate-path existence alone; do not claim the −96 gas is caused by tool usefulness or learning.
- [Full workspace run #44652186003](https://github.com/iFrankStudio/OCEE/actions/runs/44652186003) ended with its kernel job **cancelled**, not passed. It is not a full regression acceptance.
- [Source revision b3a90b3](https://github.com/iFrankStudio/OCEE/commit/b3a90b31a2bcee150e271f7dc3da3c84db11f26e) removes the invalid exact gas/tick equalities, records sham runtime gas and tick drift explicitly, and retains FOLD32, candidate-path, equal import cost and same-vs-sham assertions. **Revision CI not yet verified**. Next: verify its paired CI and inspect the printed drift; if further assertions fail, fix from native logs. Then design a schedule-matched placebo with identical pre-run tick/gas effects before making strong causal savings claims.
- Provenance: `EXEC(hash)` cannot distinguish imported code from an independently regenerated byte-identical program; use registry/candidate trajectories and avoid the label `causal_tool_used` as proof of necessary causation. P1→P2 positive transfer remains unsupported. G1/G2 **NO-GO**; E001 remains unregistered.


### 2026-11-20 — S27B 修订后的定向 CI 验收（开发集，非正式 E001）

- 已通过 GitHub Actions API 直接核验 [S27 配对运行 #44659637319](https://github.com/iFrankStudio/OCEE/actions/runs/44659637319)：head [`b3a90b31a2bcee150e271f7dc3da3c84db11f26e`](https://github.com/iFrankStudio/OCEE/commit/b3a90b31a2bcee150e271f7dc3da3c84db11f26e)，workflow **completed/success**；`paired-evidence` job 中原生 Rust 配对测试、独立 Python 配对审计及工件上传三个步骤全部 **success**。归档工件 `s27-cross-family-paired-dev`，artifact ID `14596065150`。此前检查点所称“修订 CI 未核实”由此更新为已核实。
- 同 SHA 的 [完整工作空间 #44659637322](https://github.com/iFrankStudio/OCEE/actions/runs/44659637322) 为 **completed/cancelled**，不能算全量回归通过；此处只承认定向 S27 流水线验收。该运行基于 Rust 修改 SHA，而后续文档提交 `771895c` 不改变 Rust 代码。
- 定向成功仅证明当前断言满足，**尚未从原生日志提取修订后的 sham 调度漂移数值**；不应将先前失败的 `sham=21,839` 直接冒充新运行结果。`same` 与 `sham` 的导入成本相等，但额外 WRITE tick 仍会影响 epoch 调度；`gas(same)<gas(sham)` 不能在没有调度匹配的情况下单独作为严格因果估计。`EXEC(hash)` 也不能区分继承执行与独立重新生成的同哈希代码。
- 下一研究任务：读取 #44659637319 原生 `S27 sham_control` 日志/工件，提取 cold/sham/same 成本和 tick；建立初始 tick、gas、epoch 边界严格匹配的可消费/不可消费工具对照，再探索真正跨 P1/P2 的协议无关工具复用。不要为通过实验植入正确算法模板。**G0 GO；G1/G2 NO-GO；E001 未预注册。**


### 2026-11-20 — S28 受先验限定的跨家族通用可执行工具复用，原生实测通过

- [研究设计与完整失败链](./experiments/E001/DEV-S28-CROSS-FAMILY-GENERIC-UTILITY.md)、[S28 Rust 根程序](./crates/organism-seeds/src/s28_cross_utility.rs)、[四轨迹因果测试](./crates/organism-seeds/tests/s28_cross_utility.rs)、[定向 CI #44670645988](https://github.com/iFrankStudio/OCEE/actions/runs/44670645988) **completed/success，1 passed 0 failed（218.17s）**。之前 [#44669843136](https://github.com/iFrankStudio/OCEE/actions/runs/44669843136) 和 [#44670196068](https://github.com/iFrankStudio/OCEE/actions/runs/44670196068) 是真实失败，不得隐藏：后者发现 sham 独立生成同哈希程序而版本 WRITE `Conflict` 被错误当作致命失败；研究者修复 [e7f515a](https://github.com/iFrankStudio/OCEE/commit/e7f515a12d338dbb7870ceb55bb4ffdf4f4593d7) 仅将已存在不可变版本的 Conflict 视为可恢复。
- **首次已核验的协议无关程序跨家族实际 EXEC**：Source P1 活动 VM 在公开作业字节上将研究者预置的 U32 模板通过 `RewriteOp` 改写为 `Len(Bytes)`、内容哈希封存并注册、实际 EXEC，通过后继续真实 C→B；Target P2 使用**相同 root、相同 S19/S23 预安装专家库**，继承此 P1 生成的字节长度程序，在切换 P2 专家前真实 EXEC，返回正确后继续真实 C→B。source/target root hash `a12ad82a5ab980eb14f003ea2410f7263a314ac920af4ef4e3553580eba6eca0`，工具 hash `337be40be6f438c015af1f21750325b76654ccac010628c2a0fe28d08e9d81cc`。
- target P2 world128/protocol42，实际总 A gas：**cold 22477**、**inherited 22349（import24）**、**same-cost sham 22486（import24）**、**wrong-output poison 5366（import24，未执行 B）**；D−sham **−137 gas**（约0.61%），D−cold **−128**（cold 不保证等写入调度，优先比较 sham）。cold/sham ticks 1174、inherited 1170；冷、inherit、sham 的 B gas 都是20，独立 FOLD32 正确；poison 虽真实 EXEC、却错误返回0，根 VM 拒绝切换，B gas0。**只在任务成功的世界之间比较 A 净收益**。
- 同内容 hash 的冷生成与导入继承必须由动作和版本证据区分：sham 已存程序版本，活动根仍尝试自行生成同一程序，WRITE 收到 Conflict 并按明确策略采用，不能将 sham 的 EXEC(hash) 计为 donor 因果贡献。源 P1 VM 生成与目标 P2 VM 调用均经审计验证，不是主持者手动 EXEC。
- **判据仍 NO-GO**：通用算子、模板、修改位置、接入 B 工作的长度12校验与 P1/P2 专家预设均来自研究者；单开发种子、单功能。不能将受先验限定的**功能迁移**称为开放式演化、自主发明或 H3 确认性通过。G0 GO，G1/G2 NO-GO。S28 代码 SHA `da96e136` 的 workspace 全量 CI 状态仍须独立核验。
- 下一突破口：将 `Len` 唯一候选替换为有限多个 VM 可组合的通用算子策略，要求个体**从新的公开验证任务/反馈中选择**、生成并交叉验证，最小化研究者先验；在 P1→P2 目标跨不同字节长度、不同随机种子与任务类型验证，不得把 `U32(12)` 目标作为可泛化语义。


### 2026-11-20 — S29 公开反馈选择的三算子真实跨协议验证（原生 CI 成功）

- [S29 完整研究记录](./experiments/E001/DEV-S29-EVIDENCE-SELECTED-OPERATORS.md)；[Rust 活动程序](./crates/organism-seeds/src/s29_public_transform.rs)、[原生测试](./crates/organism-seeds/tests/s29_public_transform.rs)、[GitHub Actions #44673328957](https://github.com/iFrankStudio/OCEE/actions/runs/44673328957) **completed/success，1 passed、0 failed、276.42秒**。首次 [#44673179538](https://github.com/iFrankStudio/OCEE/actions/runs/44673179538) 因测试代码 Rust 所有权错误 E0382 编译失败，通过 [4be5024](https://github.com/iFrankStudio/OCEE/commit/4be5024a2dd2a89595b0aada08a6a9d078cb0043) 修复，无修改世界 seed/实验预期。
- 世界早已发布的 `/public/E001/transform` 包含两组标注样例及 held-out 输入；S29 用同一个冻结初始 root 在真实活动 VM 中通过 `Len / XorFold / ByteAt(0)` 三候选原语对两个样例逐一比较，选择合适算子，通过 `RewriteOp` 构造不可变代码版本、受费 WRITE 注册、真实 EXEC 两例和 held-out，存储预测；研究端只在完成后用私有 oracle 校验，不把隐藏规则、seed、预期 held-out 输出传给活动 VM。根程序继续现有 S28 长度门控和 S26C P1/P2 远端专家，因此**不能把所有 B 能力归因于本次新算法选择**。
- 三组 P1→P2（protocol seed42、不同工作 world seeds）：source42→target129 选 `Len`，目标 P2 **A 23626 / ticks1183 / import25 / B20**；source43→target130 选 `XorFold`，**A 23633 / ticks1183 / import26 / B20**；source44→target131 选 `ByteAt`，**A 23630 / ticks1183 / import26 / B20**。三种工具 hash 各不相同，源 P1 实际生成/写入，目标 P2 三次成功 EXEC 且不重新生成，held-out 预测全部独立正确，远端 FOLD32 结果全部独立正确；初始 root hash 一致。
- 公平性：P2 world129 的可见注册 D 与相同程序字节、相同两次导入 WRITE 但不可见的 sham，两组导入均25、ticks 均1183；**D 23626 gas / sham 23780 gas = D−sham −154 A gas（约0.648%）**，sham 自己重新选择相同算子并在版本 WRITE 遭遇合法 Conflict。错误语义 XOR 工具迁入 target LEN 的 P2 world129，工具虽通过实际 EXEC 却未通过目标样例，**A6233 / ticks10 / B0**，停机无结果；此时不能假称“自动修复”，恢复机制有待研究。
- **科学边界**：实现的是研究者提供的有限三个算子、已知修改槽位、枚举顺序和公开样例反馈下的**任务依赖程序选择 + 跨协议可执行继承**，不是超出候选的新算子/结构创造。样本数有限，目标标签及远端专家来自环境/研究者；正式 E001 未预注册。**G0 GO、G1/G2 NO-GO**。最新完整工作空间 [#44673328915](https://github.com/iFrankStudio/OCEE/actions/runs/44673328915) 尚待核验，不能用本次定向 CI 代替。
- **下一优先问题**：当继承程序在目标公开样例下失败时，目前 fail-closed；研究“主动拒绝失配旧知识 → 重新选择其他算子 → 保存新版本 → 更新注册引用”的受计费修复，须与可见继承和等时序 sham 对照配对。随后测试需要原语组合而不仅是一元算子枚举的新任务。


### 2026-11-20 — S30 原生验证：两节点组合程序跨家族复用、失配后自主替换旧工具

- [S30 实验记录](./experiments/E001/DEV-S30-COMPOSED-CROSS-FAMILY-REPAIR.md)、[活动 VM 源码](./crates/organism-seeds/src/s30_composed_transfer.rs)、[因果测试](./crates/organism-seeds/tests/s30_composed_transfer.rs)，[定向 Actions #44677074994](https://github.com/iFrankStudio/OCEE/actions/runs/44677074994) **success，Rust 1/1，94.08s**。前置失败 [#44676720789](https://github.com/iFrankStudio/OCEE/actions/runs/44676720789)、[#44676860088](https://github.com/iFrankStudio/OCEE/actions/runs/44676860088)、[#44676983659](https://github.com/iFrankStudio/OCEE/actions/runs/44676983659) 都是静态区块参数拼接错误，后续精确图边诊断和修订解决；没有篡改模拟规则或挑选更容易的种子。
- **S30 新能力并非 S6 的重复**：将早已验证的 S6 三原语×两个二元运算形成 18 个受限组合的原生搜索，接到所有世界**完全相同的 S28/S26C root + 预置 P1/P2 专家库**，增加对携带程序的目标侧双标签再次验证、失配后的真正搜索、旧工具注册表 **CAS v0→v1**，以及用目标 seed 实际产生的不同 `transform.held_out_input` 做额外无答案输入预测。不是宿主查 hidden rule 后挑程序，P1/ P2 均经内核计费的 Exec、Write 审计。
- P1 seed1→P2 seed19：目标 A **24,066** / ticks **245** / 继承导入 **29** / B20；P1 seed10→P2 seed28：目标 A **24,063** / ticks **245** / 继承导入 **29** / B20，初始 root hash 相同，继承的是两种不同内容哈希的多节点程序。两组来源/目标独立 held-out 与额外目标特异输入预测、远程 FOLD32 全部正确。
- **等额、等起步时序的假对照**（P2 world19）A **24,714**，D 正常继承 A **24,066**，二者 import29，D−sham **−648 gas（约−2.62%）**，tick 245 vs 250；sham 复用哈希不等于来源继承，真正重新执行了探索。负迁移恢复：错误组合程序导入 target P2 world19，真实 EXEC 后被公开标签拒绝，VM 搜索出正确程序并 CAS v0→v1，结果 A **24,831** / **251 ticks** / B20，全部后续功能通过。相对 S29 错误语义仅失败，本次是有实际因果轨迹的恢复。
- **重要可比性**：S30 全配对采用 A quota=100,000 gas/epoch（早期 S28/S29 使用12,000），不能跨实验拿 gas/ticks 算性能趋势或宣称无界适应。结构语法、18 候选序列、回退/修复策略和 P1/P2 专家全部研究者预设；两配对不足以支撑统计显著性。**G0 GO，G1/G2 NO-GO；E001 未预注册。** 完整 workspace CI 另行验收。
- 下一真正研究门槛：减少固定三算子/两组合模板先验，检验任务随 episode 改变时是否能**自主提出、组合、验证**候选新图，并将选中的功能工具用于实际任务决策/通信，而不是仅为下游预置专家提供前置门控；还需跨多协议实例、资源极限、未见边界输入验证。


### 2026-11-20 — S31 首次原生实测：VM 所生成程序的输出进入真实 C→B RemoteEval 数据路径

- [完整 S31 实验记录](./experiments/E001/DEV-S31-CAUSAL-REMOTE-INPUT.md) / [活动根与两份协议专家](./crates/organism-seeds/src/s31_causal_remote.rs) / [原生反事实](./crates/organism-seeds/tests/s31_causal_remote.rs) / [GitHub Actions #44685083680](https://github.com/iFrankStudio/OCEE/actions/runs/44685083680) **completed/success，单核心原生 Rust 1 passed / 0 failed / 50.63s**。实现与测试 SHA 9b47efa10a004017dae2b1f091ca00d70766de16，后续文档不修改 Rust 测试。
- **消除真实远端请求的一个人工数据常量**：S19 P1 专家原来硬编码 [17,0,0,0,3,0]，S23 P2 专家硬编码 [3,0]。S31 修改专家：必须经真实 WorldCall::Read 消费同一份 /self/work/remote-count-le；这个对象由根活动 VM 使用 S28 研究者模板经真实 RewriteOp、不可变 WRITE、受费 EXEC 的 Len(Bytes) 程序计算 Len(values_le)，再经 Div(...,4)、PackU16Le 和受费 WRITE 得到。所有家族专家均不含固定 count3 字面量作为实际请求来源。即便曾经的 S30 多节点工具继续独立学习，真正进入 B 参数的是 **S28 生成的通用 Len 程序结果**，不能说 S30 复合程序已经直接控制 B 运算。
- 同一初始 root hash **398d412d7d5629f14a20289385e278f7073f6a3bc29848341a99a979f6e00be5**，两套专家库等额安装；P1 source seed1 自主 S30 生成的工具 hash ccbec864ef8a002aa15e07a541cfca57200fa8770f51020274f1b1d359aa43c5 真实导入 P2 seed19，且该工具仍被 P2 的目标组合验证使用。原生审计确认根 VM 的 Len 成功 EXEC **先于** prepared-count WRITE，后续专家成功 READ，原始 C SEND 可由所在模拟服务的协议解码器解析，得到 RemoteEval 的 count3 + 原始12字节 values_le。P1 **A12,893 gas / B20**；P2 **A24,289 / B20**，独立 FOLD32 成功，快照哈希重载一致。
- **严格匹配的因果反事实**：两份相同 P2 world19、相同 donor 导入，root 自主切换完成后、B 尚未执行时先断言世界 state hash 一致；再在两臂分别通过研究者**实验干预**对同一对象进行一次正常、CAS expected_version=0、相同2字节长度和**各3 A gas**的世界 WRITE，唯一差异是保留 count3 vs 改写成 count2。控制组实际发送正确 count3 + values_le，最终 **A24,292 / tick248 / B20**；篡改组实际发送错误 count2 + 完全相同 values_le，B 原生返回 **BadBody**、**B gas0**，无合法远程结果，轨迹 **A14,482 / tick110**。低 A 是失败提前终止，非性能提升。这里篡改是宿主控制的因果消融而非个体自主攻击/修复。
- 该实验为跨家族共享工具数值成果进入真实远程工作输入提供强于“已 EXEC/已封存”的**实际 on-wire 因果证据**。但 Len 模板、4字节 U32 字宽、协议专家、完整 S30 学习/修复图均是研究者先验；B 仍是**本地确定性模拟服务**，不是现实互联网；单开发配对+一次反事实不能升级正式 E001/H3，更不等于赛博生命已诞生。**G0 GO；G1/G2 NO-GO，E001 未预注册。**
- [同 SHA 完整 workspace CI #44685083679](https://github.com/iFrankStudio/OCEE/actions/runs/44685083679) 目前仍需核验最终结论；最新 S26B/C、S30 与 S31 定向 CI 通过，不能用它们冒称全量工作空间通过。历史 [S30 workspace #44677074963](https://github.com/iFrankStudio/OCEE/actions/runs/44677074963) 最终为 completed/cancelled，并非成功。
- 下一关键方向：让已有 S30 活动 VM 从输入/输出例子合成的 **多节点组合程序输出直接决定远程任务参数/操作数**；同时在更广泛任务大小、不同协议实例与错误学习工具下验证恢复，避免“只把研究者播种的 Len 插入数据链”被误读为开放式程序创新。



### 2026-11-20 — S32：跨家族学得多节点程序结果真实决定远程 B 操作数
- [完整实验记录](./experiments/E001/DEV-S32-COMPOSITE-REAL-B-OPERAND.md)；[VM 实现](./crates/organism-seeds/src/s32_composite_b_operand.rs)；[原生对照](./crates/organism-seeds/tests/s32_composite_b_operand.rs)；[定向 CI #44686613065](https://github.com/iFrankStudio/OCEE/actions/runs/44686613065) **completed/success，1 passed/0 failed/84.59s**。首次 [#44686476742](https://github.com/iFrankStudio/OCEE/actions/runs/44686476742) 静态类型失败，修订 [e619b212](https://github.com/iFrankStudio/OCEE/commit/e619b212ddbd9faefe3c2914431f577c7df7810c) 修正 WorldCall carried U32/feedback Bytes 顺序后成功。
- 冻结初始 root hash `ad0e5ab7840e26c464442fc4283accc098e04f113bd879df20542856b291b39d`，研究者预给 S30 18组合搜索、S31 Len 与两份 P1/P2 专家。P1 source seed1→P2 target19 学得图输出第四 U32 **52**；source10→target28 输出 **35**。目标 P2 两组 A **24,493** / ticks **250** / B **24** / donor import29，真实 SEND 载有四操作数，远端结果与独立 fold32 一致。程序通过真实 WRITE 和 EXEC 产生，而非 host 计算。
- 严格匹配反事实：干预前两个 P2 world19 state hash 完全一致，两条均经单次16字节 CAS 写入收费**各3 gas**，仅翻转第四 U32 的1 bit；两条 B 均成功、B gas均24、总A均24,496，真实答案 **1,534,416,407→1,534,416,415（+8）**。生成组合工具的输出**首次直接影响 B 数学答案**；不能把研究者干预算成个体自主修复。
- 错误旧程序导入 P2 world19，真实样例重检→18候选重学→CAS registry v0→v1，最终第四值52、B24、A25,259、ticks256。研究者预置的语法、任务追加规则、ABI、协议专家仍不可称为自发创造；两个开发 seed 对不支持统计确认。**G0 GO，G1/G2 NO-GO**，正式 E001 尚未预注册。完整 workspace 需独立验证。


### 2026-11-20 — S33 原生实证：从固定组合算子选择迈向有界结构深度选择
- [S33 完整研究记录](./experiments/E001/DEV-S33-ADAPTIVE-STRUCTURAL-DEPTH.md) · [VM 搜索/拓扑代码](./crates/organism-seeds/src/s33_adaptive_depth.rs) · [原生因果测试](./crates/organism-seeds/tests/s33_adaptive_depth.rs)。首轮 [CI #44689632213](https://github.com/iFrankStudio/OCEE/actions/runs/44689632213) 因新公共任务 Read ACL 返回 Denied；研究者只允许明确命名的 `/public/E001/compose-depth` 路径，不开放任意 public 读写；[CI #44689804452](https://github.com/iFrankStudio/OCEE/actions/runs/44689804452) 已通过；追加全部18个浅层候选确实耗尽的断言后，**[增强原生 CI #44690352743](https://github.com/iFrankStudio/OCEE/actions/runs/44690352743) completed/success，Rust 1 passed / 0 failed / 112.87s**。
- S33 测试环境在每个世界初始化时由研究者安装只读公共两例标签任务（不自动成为普通世界任务），隐藏函数按 world seed 选规则：偶数只执行既有 S6 两一元+一二元组合；奇数需在此基础上追加 Len(input) 再 Add。活动 VM **不知道 seed 及隐藏规则**，仅真实 READ 公开两个输入/输出样例。相同初始 root `23a7f9d7bc49a6c77b02fe0d6d0beb06d862c63f721786b3bd05dac5e0911950`，程序在研究者预给的 **18浅层 + 18深层共36有界候选**中逐一实际 RewriteOp/WRITE/EXEC，选中后继续 S32 的跨家族共享真实 B 第四操作数路径。两类拓扑本身由研究者提供，未出现任意新图结构发明。
- P1 seed1→P2 seed19：正确生成真正的深层 **6条指令**图，程序 hash `ba26376fb3a4613f0f0090125ef1c8349993c965087792c068a302b3a143745b`，import32 A，P1 A27,785，目标 P2 **A24,833 / 250 tick / B24**。P1 seed10→P2 seed28：正确选择浅层 **4条指令**图，hash `81cd576411f67e85c2aa821146613e2db1f10dc5ff51e7c04ad92b12ee428c95`，import29 A，P1 A20,979，目标 P2 **A24,822 / 250 tick / B24**。同初始 root、不同实际内容哈希的图，目标 P2 重新验收自己的标签并在新输入中生成真实远端 FOLD32 操作数，未重新生成已继承的版本；实际 C wire 和独立 B result 均经原生核验。
- 关闭深层结构、保留相同公共深层任务的 P2 world19 **A20,614 / 44 tick / B0**；增强断言确认尝试耗尽所有**18个浅层候选的真实版本写入**，不是中途 trap/停止导致的虚假负例。等成本可见继承 D **A24,833** vs 相同源程序字节但注册键不可见的 sham **A39,830**，两组导入均32 A、B均24，净省 **14,997 A gas**，D tick250 vs sham291；只可作该开发配对同起跑条件下的搜索复用收益，不可称无调度混杂的泛化成本证明。错误浅层程序导入深层世界，VM 目标双标签拒绝/搜索深层/CAS 注册 **v0→v1** 后真实 B成功，**A39,924 / 292 tick / B24**。
- 科学边界：世界公共深度任务由**测试宿主注入**，新只读 ACL 为受控开发扩展；18+18 结构和搜索序列、U32 ABI、P1/P2 专家、S32 把学习程序输出追加为第四操作数的策略全部由研究者固定。仅两个探索性 seed 配对 + 一组 sham/结构消融/错误恢复；不支持开放式演化或正式 E001 H3。**G0 GO；G1/G2 NO-GO。** 最新完整 workspace CI [#44690352570](https://github.com/iFrankStudio/OCEE/actions/runs/44690352570) 仍在执行，不能把定向成功冒充全量回归成功。
- 下一研究门槛：在保持严格代码静态验证、资源计量和真实因果对照的前提下，逐步降低两种研究者预置拓扑的先验，开放可组合任意深度/边的受限类型化语法，研究选择搜索预算与停止策略；另需跨多种 seed/任务、未知协议家族和真正外部服务做能力迁移，而非只用固定的模拟世界。


### 2026-11-20 — S34：同一初始骨架的原生 VM 增量图构造（三深度）与真实远程 B 因果路径
- [完整 S34 原生记录](./experiments/E001/DEV-S34-INCREMENTAL-GRAPH-GROWTH.md) · [图生成器](./crates/organism-seeds/src/s34_incremental_depth.rs) · [通用 VM 扩展原语](./crates/organism-vm/src/lib.rs) · [跨家族/消融测试](./crates/organism-seeds/tests/s34_incremental_depth.rs) · [**定向原生 Actions #44693172156**](https://github.com/iFrankStudio/OCEE/actions/runs/44693172156) **completed/success：VM 独立编辑器 1/1 + S34 世界实验 1/1（126.57s）**，所验收代码 SHA [fa45749](https://github.com/iFrankStudio/OCEE/commit/fa45749a9cce826f86b5f0f0fdb532b7bd7edde0)。初版 VM 单元测试 E0425 漏辅助工具与随后延伸代码 hash/WRITE 字节不一致产生的 InvalidCode **均被保留为真实工程失败**，修复未改内核校验。
- S33 仅选择两个研究者预制完整拓扑；S34 新增可计费 `Op::AppendInstructions(source,block,serialized_instructions,return_id)` 作用于已通过全静态校验的程序 Return 区块，返回新的规范序列化代码，完整验证类型/寄存器引用/大小/控制流，拒绝不合法编辑；核心单测涵盖多类失效和合法扩展。活动 VM 从**一个4条指令的共同初始图**、研究者给定的18种基础算子和最多2层 `Len(input);Add(acc,len)` 类型化片段，逐一构造54候选并基于两个公开输入/输出标签决定采用0、1还是2层；不是宿主预先给三个完整可执行模板。初始 root hash 在P1/P2三个来源+三个目标世界均为 `18e02a10bb3d641701bf5fa7908cd4b1d1f2ae15056dc7ec794e1cf6c203a5b8`。
- P1 seed1→P2 seed55 **0阶段/4指令**：来源 A13,526、P2 **A24,605/B24/tick250**、导入29 A，最终工具 hash `ccbec864ef8a002aa15e07a541cfca57200fa8770f51020274f1b1d359aa43c5`。P1 seed19→P2 seed73 **1阶段/6指令**：来源 A22,886，P2 **A24,620/B24/tick250**、导入32 A，hash `ba26376fb3a4613f0f0090125ef1c8349993c965087792c068a302b3a143745b`。P1 seed37→P2 seed91 **2阶段/8指令**：来源 A32,889，P2 **A24,625/B24/tick250**、导入35 A，hash `b66d0084a3106dafbee2ae2426404c5d6c016c7016a08baf23ff13e920647931`。三组均跨家族真实导入、目标双例再验收/未见输入 EXEC、真实封存第四 B U32、独立 Len 生成 count4、专家发送 C 帧、B 计费24并独立 fold32 正确。
- 限制最多1追加阶段的 seed91 P2：真正尝试完**全部36个**版本 WRITE 仍 **A26,135/tick80/B0**，不属于提前故障。两阶段正确可见继承 P2 **A24,625/tick250/B24** vs 同版本导入费35的不可见 registry sham **A44,926/tick327/B24**，节约 **20,301 A gas**。错误一阶段知识导入两阶段 P2，真正目标反馈重学/受费 CAS registry v0→v1 后继续正确 B：**A45,019/tick328/B24**。
- **科学边界**：54 候选、最大2追加阶段、片段操作和搜索顺序、两份协议专家及 FOLD32 工作 ABI 均为研究者预置；开发性两标签任务由实验宿主提供，只涉及本地模拟 C/B 服务，非开放式发明、无限递归程序生长或预注册 E001/H3。**G0 GO；G1/G2 NO-GO。** 与本次代码对应的全量 workspace [#44693172200](https://github.com/iFrankStudio/OCEE/actions/runs/44693172200) 尚需独立核验；不要用定向1/1冒充全量通过。


### 2026-11-20 — S35 语义歧义复现与公开反例纠正：防止真实 B 成功掩盖学习错误
- [S35 完整研究及原生结果](./experiments/E001/DEV-S35-COUNTEREXAMPLE-AMBIGUITY.md) · [活动 VM 第三标签验证](./crates/organism-seeds/src/s35_counterexample.rs) · [因果对照测试](./crates/organism-seeds/tests/s35_counterexample.rs) · [**原生定向 CI #44695944146**](https://github.com/iFrankStudio/OCEE/actions/runs/44695944146) **completed/success，Rust 1/1 / 136.54秒**，代码 SHA [0a5cc6f](https://github.com/iFrankStudio/OCEE/commit/0a5cc6fcf0f78114d9cb588b8893228f3f5bc254)。首次 [#44695753225](https://github.com/iFrankStudio/OCEE/actions/runs/44695753225) 编译和世界执行正确，但测试错误地把后续独立生成的 S28 Len 版本算成搜索候选；修复计数区间后通过，未改学习行为或任务内容。
- **可构造的真实假泛化**：给研究者预置的两公开样例 `[1]`、`[2,0]`，三原语 Len、XorFold、ByteAt 输出都一致；旧 S34 学习器选 `Len+Len` 等价程序、**第四 B 操作数10 而隐藏规则应为48**，依然正常远程 B FOLD32（B24、A25,034）。因此仅凭真实 C wire/B 算术正确不足以声明所学规则语义正确。这一失败不是纸上推理，而是旧控制在原生世界真实完成 B 的实测。
- S35 保持同54候选、三阶段增量图和 P1/P2 专家，新增**精确白名单只读** `/public/E001/compose-counterexample`，活动 VM 在两个标签通过后才按需 READ 环境提供的第三个公开标注 `[7,8,9]` 并真实 EXEC 候选。失败继续搜索；已继承程序也必须同样第三次验收，错误继承重新搜索并按已有 CAS 注册替换。缺失挑战时按研究者既定策略 **fail closed**、B0。挑战标签由研究者环境提供，非个体自行查询/发现，不能视作无监督环境探索或开放式发明。
- 初始 root hash 在所有来源 P1/目标 P2 世界保持 `f8467165e6acb3fdbef100abafb3921e7d1052134b340e73afa8611f927551c1`。P1 seed1→P2 seed55 **0阶段**目标 A24,991/B24/导入29；seed19→73 **1阶段**目标 A25,008/B24/导入32；seed37→91 **2阶段**目标 A25,015/B24/导入35。来源候选 WORLD WRITE **2/20/38**（不计后续单独 Len 工具），挑战 READ 来源各2、目标各1；三组实际目标未标注输入、B 真正 FOLD32 与快照重载正确。
- **旧两样例知识的跨家族拒绝与修复**：旧 P1 活动 VM 生成错误但前两标签拟合的程序，导入新 P2 seed55（import29）；目标仍需第三标签，实际驳倒旧程序并在本地搜索、CAS registry **v0→v1**，正确第四操作数恢复**48**，最终 **A26,324 / 264 ticks / B24**，挑战 READ 共3次。缺失证据世界 **A7,785 / 12 tick / B0**，不假造结果。
- **等成本、同初始状态假注册表**（两阶段 P2 seed91）：两臂导入字节/付费相同 **35 A**，可见继承 **A25,015/tick252/B24**，隐藏注册位置 sham **A45,461/tick332/B24**，净节约 **20,446 A**（约45.0% sham 总 A），双方真实 B 正确。只可归因于避免同设置下的候选重搜，不得推广为普遍性能优势。
- 科学边界：第三标签为研究者提供的**额外监督信号**，并非 VM 自主构造信息增益最高的环境查询；两条不足、三条足够仅在此预设计别名任务成立，未进行统计确认或真实互联网验证。结构、搜索和修复算法及专家仍研究者先验。**G0 GO；G1/G2 NO-GO。** 同 SHA 完整 workspace [#44695944307](https://github.com/iFrankStudio/OCEE/actions/runs/44695944307) 另行核验，不以定向成功代替。
- 下一有效研究突破点 S36：让个体针对多个仍可行假设在可访问环境中**自行选择有区分度的探针输入**，而非研究者预先给定第三标签；必须将输入请求、环境输出、试错资源成本和未知输入判据完整记录，同时区分环境本身提供答案与个体主动选择何时/问什么。


### 2026-11-20 — S36 原生突破：主动选择区别性输入与环境交互，缓存单次查询证据用于跨家族纠错
- [完整 S36 开发研究](./experiments/E001/DEV-S36-ACTIVE-PROBE-LEARNING.md) · [活动 VM](./crates/organism-seeds/src/s36_active_probe.rs) · [内核受限 Query](./crates/world-kernel/src/lib.rs) · [跨 P1/P2 与对照](./crates/organism-seeds/tests/s36_active_probe.rs) · [**原生定向 Actions #44697820767**](https://github.com/iFrankStudio/OCEE/actions/runs/44697820767) **completed/success，Rust 3 passed / 0 failed / 119.97s**，验收代码 SHA e0e7bf7。初始 root 全家族同 hash `0c4a108457eb67dcc5c86c26d0b3528a376e9139dd6a2e0ae9a5ddf232410a37`。
- 与 S35 预先 READ 第三条公开标注不同，S36 普通世界仅公布**两条可能语义混淆的训练样例**。活动 VM 从研究者给的 `[1]`、`[7,8,9]` 两个候选输入中，按 Len 与 XorFold 的模型预测是否分歧来选择实际探针：第一例无分歧，第二例 Len3/XorFold6 有分歧，因此发出真实、由内核记录的 `WorldCall::Probe` / `Action::Probe{input:[7,8,9]}`；输入在动作发起前无标签。内核仅开放 task_id 精确匹配的 S36 本地模拟环境，输入1–8字节，每世界只成功查询一次，受费52 A，并在 probe_count 状态、snapshot 和 audit 固化。VM 付费将得到的 TransformExample 写到 `/self/evidence/s36-active-probe`；后续候选复用本次证据，继承程序也不能跳过新环境的实际探测。
- 三深度 P1→P2：0阶段 source1→target55（P1 A14,502，P2 **A25,445 /254 ticks/B24**，导入29 A）；1阶段 source19→target73（P1 A23,868，P2 **A25,462/254/B24**，导入32）；2阶段 source37→target91（P1 A33,877，P2 **A25,469/254/B24**，导入35）。来源每组实际版本搜索2/20/38，真正查询各一次并收费52 A，目标独立查询各一次且正确使用 P1 真实导入的不可变程序作为真 B 第四操作数，未见数据 oracle、原生 C/B 与快照均正确。
- 两标签旧控制仍真实 B24，但第四操作数 **10 vs 正确48**（A25,031），展示协议/B 成功与语义正确的区别。错误旧 P1 工具虽然通过两标签，S36 P2 一次主动 Query 后识别失配/本地重学/CAS registry **v0→v1**，恢复 B24（**A26,776 /266 ticks**）。查询预算预先占满时 VM 探测遭内核 Quota 并 fail-closed **A8,151/tick13/B0**；直接接口测试有 Denied/BadRequest/Ok/Quota与快照覆盖。更换探针建议顺序，VM 会优先使用首次有区分度的输入，真实 B24。
- 同导入35 A的可见继承 vs 不可见注册 sham：P2目标 **A25,469/tick254/B24** vs **A45,914/tick334/B24**，节约 **20,445 A gas**，双方都真正主动探测一次而不是仅凭复制哈希判断继承。
- **新增动作编译回归必须承认**：[首次完整 workspace #44697820771](https://github.com/iFrankStudio/OCEE/actions/runs/44697820771) 与 [S28 #44697820921](https://github.com/iFrankStudio/OCEE/actions/runs/44697820921) 因旧 S28 测试诊断 `match Action` 未覆盖 `Probe` 编译 E0004 失败；[667f9afd](https://github.com/iFrankStudio/OCEE/commit/667f9afd7c8ab804b3352864060d28f1081c16c3) 仅补诊断 enum 分支，修复后的 S28 [#44698286230](https://github.com/iFrankStudio/OCEE/actions/runs/44698286230) 与完整 workspace [#44698286285](https://github.com/iFrankStudio/OCEE/actions/runs/44698286285) 需另查实际 CI，不能用S36定向成功冒充回归完成。
- **科学界限**：仍是两个研究者预设的假设/输入、单次可信本地查询、封闭54候选、S30 预置失配恢复、固定协议专家与 B 模拟器；并非自主构造信息增益最优输入、开放式自我演化、实际互联网自动适配或 E001 H3 确认。当前缓存没有任务指纹/认证，任务更替/伪缓存是下一项 **S37** 最有价值的可证伪失败模式。**G0 GO，G1/G2 NO-GO。**


### 2026-11-20 — S37 内核签发回执/任务哈希验证：原生证明缓存伪造、CAS篡改和任务漂移风险
- [S37 完整实验与原生证据](./experiments/E001/DEV-S37-TASK-BOUND-EVIDENCE.md) · [活动 VM VerifyProbe 门](./crates/organism-seeds/src/s37_evidence_guard.rs) · [受费内核校验](./crates/world-kernel/src/lib.rs) · [7 项原生研究/因果实验](./crates/organism-seeds/tests/s37_evidence_guard.rs) · [**定向 CI #44699875380**](https://github.com/iFrankStudio/OCEE/actions/runs/44699875380) **completed/success，7 passed/0 failed/147.39s**，验收代码 SHA [`0834e07`](https://github.com/iFrankStudio/OCEE/commit/0834e07c49ddd89ddef90aba1961f34291511a97)。全 workspace [#44699875373](https://github.com/iFrankStudio/OCEE/actions/runs/44699875373) 仍须独立验证，不得声称所有回归已过。
- 内核仍只有S36明确授权的单次短输入 Probe，成功时额外内部记录 `ProbeRecord{task_hash:SHA256(public_task_bytes),receipt:exact_issued_TransformExample_bytes}`；新 `WorldCall::VerifyProbe(Bytes)` 由活动 VM 在每次使用本地缓存前真实发起，内核对原始签发记录和**当前任务字节哈希**共同校验，正常/错误一致收费2 A、绝不返回隐藏答案本身；无记录 NotFound、失配 Conflict、证据过长 BadRequest。ProbeRecord 不是 `/self` 可写内容且能随世界 snapshot 重放，严格意义是可信内核 provenance 比对，并非密码学签名。
- P1→P2 0阶段 seed1→55：来源 A14,645、目标 **A25,557/tick255/B24/导入29 A**；1阶段19→73：来源 A24,011、目标 **A25,574/tick255/B24/import32**；2阶段37→91：来源 A34,020、目标 **A25,581/tick255/B24/import35**。同初始 root `40a28629186f555211a745329bab03b1a488c9c581f591dacf9b853d126993d2`，每世界真正 Probe 一次收费52 A、保存证据一次，来源 Verify 两次/目标 Verify一次，正常未见输入第四操作数与 C wire / B FOLD32 均正确。错误旧知识导入后真实 Verify/重学/CAS registry v0→v1 修复 **A26,950/tick269/B24**；Quota用尽时拒绝 **A8,232/tick13/B0**。
- **第一次真实因果伪证据对照**：只用受费 `/self` Write 伪造input `[7,8,9]`→output6，旧 S36 **真实B24**，第四操作数10而正确48；S37 内核无签发记录 Verify NotFound→**B0**。**查询后CAS篡改对照**：成功 Probe 后用同权限付费CAS把缓存值改成10，旧S36第四操作数43而正确48仍B24，S37 第二个候选 Read 时 Validate Conflict→**B0**。**环境真实规则变化对照**：研究者把 v1 的隐藏oracle和公共标签切换为 v2 oracle `2×Len`，旧 S36 输出旧值48而新正确10仍B24，S37 因任务指纹不同 Verify Conflict→**B0**，无法在当前单次预算下自动再次探测，直接尝试返回 Quota且不收费。
- S37 同费可见继承/不可见 registry 对照（P2 seed91）导入各35 A、B均24：**A25,581/tick255 vs A46,057/tick336**，节约20,476 A。旧模块 S28 诊断的新增 `VerifyProbe` 枚举分支已在 [0efbfad](https://github.com/iFrankStudio/OCEE/commit/0efbfad37ecd246f05fa05edd170660ab6ea45c8) 加入，避免此前 S36 新增 Probe 时发生的 E0004 回归。
- **科学边界**：固定54候选和探测策略、内核特权记录与本地模拟 C/B、研究者提供伪造和任务变更事件，不等于自由自修改/真实服务学习；不具备经过验证的密码学证明、并发TOCTOU安全、对任务漂移的自主恢复。此阶段证明限定条件下的缓存证据污染检测与错误任务拒绝。**G0 GO；G1/G2 NO-GO；正式E001尚未预注册。** 下一研究应将任务版本/配额能力与新的合法查询策略关联，在规则改变后可安全重新学习而不无限重试。


### 2026-11-20 — S38：可验证任务升级授权、受费第二次探针、CAS证据更新及真实B能力恢复
- [S38 完整研究与原生因果试验](./experiments/E001/DEV-S38-TASK-VERSION-RELEARNING.md) · [活动VM](./crates/organism-seeds/src/s38_task_recovery.rs) · [内核受费RenewProbe](./crates/world-kernel/src/lib.rs) · [4项原生实验](./crates/organism-seeds/tests/s38_task_recovery.rs) · [**定向CI #44702623338**](https://github.com/iFrankStudio/OCEE/actions/runs/44702623338) **completed/success：4 passed/0 failed/49.40秒**，验收源码 SHA [`2383cf3`](https://github.com/iFrankStudio/OCEE/commit/2383cf30daa2143151e27f9aa0b71fbe618a66f8)。初始root hash `d15bb3aa1985671fe18c881647c3a04e089323a569c00171837df83c390106f2`。
- S37在旧证据与当前任务指纹不同的 VerifyProbe Conflict 时只会B0；S38额外在原活动VM中传递旧真实回执给 `WorldCall::RenewProbe`，内核仅允许严格研究者预置的 `/public/E001/compose-depth` v1→v2、当前公开版本1、尚未续期且旧回执与内核历史记录一致时发放**一次额外许可**（收费7 A）；再通过活动VM的Len/XorFold区别度比较选择新的[7,8,9]第二次受费Probe（52 A），根据v2标签6以真实CAS更新缓存 v0→v1，最终经过额外 VerifyProbe 才允许真实Exec。图静态验证曾拒绝不设限的恢复循环，现通过独立终结验证块**消除无界回路**。同世界最多2次成功Probe、最多1次续期；无合法凭证/不合版本/篡改均禁止额外查询。
- **真正 v1→v2 原地恢复**：P2 seed55在首次成功Probe后由研究环境改变公开任务版本与隐藏oracle为`2×Len`，VM付费Renew7+Probe52，重算未见字节的第四数为**10**，真实B24，最终 **A26,653 /260 ticks**。旧S37对照 **A8,359/B0**，未被免费重置配额。
- **实际P1→P2代码迁移后重新学习**：来源P1 seed1真实学得v1工具hash `ccbec864ef8a002aa15e07a541cfca57200fa8770f51020274f1b1d359aa43c5`（P1 A15,096），导入目标P2 seed55（import29 A）；P2任务变更后重新查询拒绝旧知识，生成新图hash `39c1196f961fc3538686460216d0b0037ed950ec03ac6c5aa5add54addbcf098`，注册表受费CAS v0→v1，真实B24，目标 **A26,945 /267 ticks**。源目标都是真实协议模拟任务而非宿主直接输出答案。
- **伪造、篡改旧证明的拒绝**：不存在内核实际签发时Verify NotFound、不进入续期；真实Probe后再在下一候选READ前以受费CAS污染缓存，Verify Conflict、Renew因旧回执不匹配内核记录返回Conflict且收费7 A，**不授新预算/B0**。直接内核断言：v1续期Denied，v2真旧回执批准一次且收费7 A，第二次Probe耗费52 A返回新标签6，重复续期/第三次Probe均Quota；快照、状态哈希完整重载。
- **工程失败诚实归档**：[首轮 #44702007308](https://github.com/iFrankStudio/OCEE/actions/runs/44702007308) workflow YAML冒号格式错误；[Rust #44702131907](https://github.com/iFrankStudio/OCEE/actions/runs/44702131907) VM验证拒绝unbounded cycle；[改进后 #44702268077](https://github.com/iFrankStudio/OCEE/actions/runs/44702268077) 2/4通过、两项对照测试错误：P1 heldout不能写成固定输入，在途真实Probe响应不能视为被篡改磁盘缓存。修正后 [#44702623338](https://github.com/iFrankStudio/OCEE/actions/runs/44702623338) 4/4成功，没有放宽世界安全验证或替换实验隐藏规则。
- **科学界限**：只对研究者授权的确定性v1→v2单次变更与封闭54候选提供有限恢复，内核有可信旧回执/任务hash，非真实互联网、多任意任务或不受限自主演化。**G0 GO；G1/G2 NO-GO；E001尚未预注册。** 同源码完整 workspace [#44702623319](https://github.com/iFrankStudio/OCEE/actions/runs/44702623319) 仍需独立查看，不以定向通过冒称全量回归。下一步可优先研究多次任务漂移与原子任务版本绑定，减少研究者固定的续期协议，严格限制验证后与真实B之间的TOCTOU。


### 2026-11-20 — S39 原生验证：活动 VM 签发任务版本栅栏，真正 Send 和 C/B 消费处双重防TOCTOU
- [S39 完整记录](./experiments/E001/DEV-S39-REMOTE-TASK-FENCE.md) · [活动VM建栅栏](./crates/organism-seeds/src/s39_task_fenced_remote.rs) · [内核发送/服务消费校验](./crates/world-kernel/src/lib.rs) · [四项原生因果对照](./crates/organism-seeds/tests/s39_task_fence.rs) · [**原生定向 Actions #44704534899**](https://github.com/iFrankStudio/OCEE/actions/runs/44704534899) **completed/success，4 passed /0 failed /104.06秒**，验收源代码 SHA [`2f9a0b7`](https://github.com/iFrankStudio/OCEE/commit/2f9a0b71bddbb559714763790005255f2a15e58b)。同期S38定向回归 [#44704534991](https://github.com/iFrankStudio/OCEE/actions/runs/44704534991) success；同SHA完整 workspace [#44704534900](https://github.com/iFrankStudio/OCEE/actions/runs/44704534900) 仍须独立确认。
- 活动根延续S38主动学习/受费新任务续期和真实第四B数生成，在 `s32_values_saved` 成功后**重新读取本地证据**并向内核 `ArmTaskFence(evidence)`；只有当回执与**内核成功Probe签发记录**及当前公有任务hash完全一致时才付费3 A创建 actor 版本栅栏。`Action::Send`在发送入队前根据该指纹比对当前公开任务，失配Conflict且不扣A/带宽；通过后把待处理消息id绑定同一版本并存入世界state。`process_service_inbox` **在真实C/B服务调用之前再次校验该消息版本**；过期即丢弃并存带id/task_hash的`TaskFenceDrop`，不消耗B gas；正常成功消费或丢弃后清理版本记录，待带宽不足延期时保留。无栅栏旧世界行为保持不变，三类新增状态未启用时省略JSON序列化。
- 冻结初始 root hash `23c42373289b15c801a9b48d5c2599d478d6da447ac9750c0d1d1de8410c651e`；正常P1 seed1→P2 seed55 由真实P1生成程序、目标付费导入 **29 A**、源目标独立主动探测/Arm、专家真实Send与C/B执行，P1 **A15,237**，P2 **A26,150**，双方 **B24**，未发生错误丢弃。正常P2在首次Probe后切换v2，S38途径真正付费Renew7/新Probe52/缓存CAS/图重学，S39Arm新任务哈希后B24，**A26,794 /262 ticks**。
- **真实Send前竞态**：已观测到S39真正Arm Ok后由研究者把任务从v1改到v2（第四U32应由48变10），接下来的实际Send由内核Conflict拒绝，**B0**；无栅栏S38对照B24但计算旧数48，语义错误。
- **真实在途竞态**：两组P2世界实际发送可独立解码的4 U32 REMOTE_EVAL并确认消息**id=26**尚在pending_deliveries；任务此时转v2，S39在C/B处理前发现版本不匹配，`TaskFenceDrop(message_id:26)`记录并B0；无栅栏S38同id服务事件真实消耗B24、第四数48而v2正确10。该区别由原生消息状态/服务事件/B gas审计验证，而不是依赖仅有最终返回值。
- 内核直接验收：假签发凭证Arm NotFound/收费3 A，真实Probe后伪证据Arm Conflict/收费3 A，合法证据Arm Ok且快照一致，任务变化时不能用旧回执重新Arm，直接绕开VM构造Send仍Conflict、不扣A或C带宽、不入队。**安全边界**：本地确定性内核信任成立才支持SEND与消费处原子检查；对Arm前消息、已完成B操作、恶意外部服务、真实网络分布式事务和被拒请求的幂等恢复暂无验证。研究者提供任务/专家/语法和触发时机。**G0 GO；G1/G2 NO-GO；正式E001未预注册。**
- 下一阶段 S40 最有意义的可证伪问题：在受保护消息被丢弃后**何时以及如何安全恢复任务**，既要重新查询并使用新语义，也要证明没有重复B扣费、重放旧消息或凭空成功；应减少对预设重试策略的依赖并考虑服务执行后才失效的不可回滚边界。


### 2026-11-20 — S40：从真实过期消息丢弃到个体自主发起一次恢复、重新学习并正确B只计费一次
- [完整S40研究与失败链](./experiments/E001/DEV-S40-DROPPED-REQUEST-RECOVERY.md) · [P2活动专家检测失败并请求恢复](./crates/organism-seeds/src/s40_dropped_recovery.rs) · [内核TaskFenceDrop/FenceStatus/RestartRoot](./crates/world-kernel/src/lib.rs) · [4项真实跨家族/幂等对照](./crates/organism-seeds/tests/s40_dropped_recovery.rs) · [**定向原生 Actions #44710108675**](https://github.com/iFrankStudio/OCEE/actions/runs/44710108675) **completed/success，4 passed /0 failed /99.83s**，验收代码 commit [6480ac3](https://github.com/iFrankStudio/OCEE/commit/6480ac3f986718d6f41089df74e36e5b882a4507)。同期完整 workspace [#44710108719](https://github.com/iFrankStudio/OCEE/actions/runs/44710108719) **尚未确认最终结果**，不得用4/4冒称全量通过。
- **真实状态链**：P2 world55第一次完整真实RemoteEval message_id26经过S39任务栅栏发送，研究者将环境隐藏oracle/公开任务变v2，C/B消费前确实TaskFenceDrop，旧消息B0。活动P2专家经128轮有界Receive后真实FenceStatus Ok并发出一次RestartRoot，内核仅在丢弃真实、actor相符、当前任务指纹和丢弃observed_hash相同、且从未重启时收8 A，从首次内容寻址root版本重新开始；旧探测历史/旧工具注册不删，清理明确上次工作派生输出/家族分类，然后S38由真实旧回执续期7 A、第二次Probe52 A、缓存v0→v1的CAS、旧图被新v2标签拒绝、工具注册v0→v1的CAS，最终v2正确第四操作数10、新S39栅栏和第二条真实C/B消息。实际只有第二条B气费24，旧请求ID26未消费/未重复B。P2 **A33,213 /883 ticks/B24仅一次**；300后续tick无额外B，活跃程序版本 **0→1→2→3**，冻结root `bf7fba37c48695959ecf0b014fbf81227069222e5f8df463b548dcfcb828b839`。
- **真正来源迁移**：P1 seed1起源主动VM实际生成v1图、B24并消耗 **A15,415**；导入P2 seed55付费 **29 A**，P2旧工作真实发送/丢弃，运行中的专家请求重启，目标独立重新获v2标签/换图/再次发新任务，**A32,110/575 ticks/B24仅一次**，来源与目标完整root字节相同。没有由研究宿主直接替换程序或发送答案。
- **拒绝/正常对照**：没有TaskFenceDrop不能请求恢复；已经恢复一次再次RestartRoot返回Quota；真实丢弃后若任务变回原字节，内核根据当前task hash与drop observed hash不同返回Denied；没有故障正常P2仍B24。老S39受栅栏但无恢复时同样在途丢弃后B0；原S38无栅栏则按既有结果B24但操作数旧值错误，因此不能把S38当作B0负对照。
- **单调CAS重放和派生状态**：S26原路由固定`expected_version=0`，重启后`/self/active`已为v2，因此S40添加只读、受费的 `/sys/active-version`，活动VM读取真实版本后普通CAS切换v2→v3，不复位/绕开安全检查；内核重启仅对本任务硬编码的held-out临时结果、两份远程B工作对象及`/self/discoveries/framing-family`做存储计量恢复，保留工具注册和内核回执以再次核验。原生日志显示不清理framing-family时在重新Arm新栅栏后因该对象创建冲突而无法发送；修订仅补清理清单。
- **工程失败如实保留**：[#44707304132](https://github.com/iFrankStudio/OCEE/actions/runs/44707304132) 与 [#44708193096](https://github.com/iFrankStudio/OCEE/actions/runs/44708193096) 都只有2/4，暴露重复活动CAS/派生数据重用缺陷；[#44709096078](https://github.com/iFrankStudio/OCEE/actions/runs/44709096078) 为诊断用格式化引入Rust E0277编译失败；[#44709246739](https://github.com/iFrankStudio/OCEE/actions/runs/44709246739) 精确证实已复查、重新Probe、CAS重学、Arm成功但Write(/self/discoveries/framing-family)Conflict；[#44709627497](https://github.com/iFrankStudio/OCEE/actions/runs/44709627497) 实现已3/4，最后一项对照误用无栅栏S38并期待B0，改成有栅栏S39后 [#44710108675](https://github.com/iFrankStudio/OCEE/actions/runs/44710108675) 4/4原生成功。没有降低原定B仅一次、未改变隐藏v2规则、未放宽内核安全验证。
- **科学界限**：128轮等待、1次恢复、指定清理清单、已知v1→v2协议、18×3预设程序搜索、两专家、可信单内核C/B服务均研究者预置；能证明这个故障模型里一次因果性的有限自修复与无重复B消费，不能称开放式赛博生命、任意网络服务幂等、无界自主学习或确认性E001。**G0 GO；G1/G2 NO-GO。** 下一阶段重点减少硬编码超时/派生状态白名单，建立受限、可计费的环境失败反馈，并研究多回合因果学习是否真正迁移到未知任务而非只适配研究者已定义的v2语义。

### 2026-11-20 — S40 CI 补充验收（独立查询，不代表新增能力）
- 通过 GitHub Actions jobs API 复查同源码的 [完整 workspace #44710108719](https://github.com/iFrankStudio/OCEE/actions/runs/44710108719)：`kernel` 作业 `completed/cancelled`，因此**全量回归没有通过的证据**；不能把 [S40 定向 #44710108675](https://github.com/iFrankStudio/OCEE/actions/runs/44710108675) 的 `native-dropped-recovery completed/success`（4/4）提升为全 workspace 成功。
- 本次没有修改实验行为、隐藏规则或增添测试，也未重复触发耗时 CI。后续优先检查全量作业取消原因及是否由新提交抢占；如需回归，应只对最新源码进行一次必要的全量运行，不能把 cancelled 视为测试失败或通过。
- 下个可证伪实验优先对 S40 的固定 128 轮等待与硬编码派生状态清理做受费、有限的故障归因；同时保持旧消息 B0、新消息 B24、版本/证据 CAS、快照和负面对照不变量。当前 G0 GO、G1/G2 NO-GO，正式 E001 未启动。


### 2026-11-20 — S41开发性候选，原生运行器未分配；不记为新突破
- [S41完整待验规范及阻断证据](./experiments/E001/DEV-S41-EARLY-DROP-LIFETIME.md) · [受限P2早期故障检测](./crates/organism-seeds/src/s41_early_drop.rs) · [4项预设因果对照](./crates/organism-seeds/tests/s41_early_drop.rs) · [原生工作流](./.github/workflows/s41-early-drop.yml)。S41是在已证实S40一次恢复B24的基础上，尝试把实际消息丢弃到自主RestartRoot之间的检测延迟从等待完128轮有界Receive，改为每次真实 `Receive Empty` 后先付费`FenceStatus`；无滴落继续原有循环，禁止内核自动重启。`FenceStatus`从此前失败不收费，改为**Ok和Empty均收1 A gas**，受既有有限重试上限保护。
- `RestartRoot`由固定5个单一路径的清理升级为固定的生命周期前缀 `/self/results/`、`/self/work/`、`/self/discoveries/`，只清除当前owner属于actor的前回合派生对象，并相应更新storage_used；保留`/self/evidence/`历史证据、`/self/tools/`可复核工具注册、`/self/versions/`不可变程序、`/self/notes/`及其它非匹配前缀。**这种分类仍是研究者给出的策略，不能称个体自行发现因果状态依赖。**S40原monotonic CAS恢复根被抽成可复用函数，不能暗改S40旧实验。
- 已冻结4项待验原生场景：(1) 同seed的S41/S40从真正TaskFenceDrop审计tick到RestartRoot审计tick的延迟，S41至少快40 tick，两个仍B24仅一次；未知名称的结果/工作/发现对象均被清理，独立持久私有对象不被删除；(2) P1真实旧工具跨家族导入P2 v2，新图真实CAS、正确B24；(3) 无drop时受费Empty查询但B正常、无成功恢复；(4) 历史drop但任务hash回滚时仍不得RestartRoot。每场要求世界状态hash与snapshot重载一致。
- **原生 CI 当前被外部运行条件阻断**：[S41 #44748358687](https://github.com/iFrankStudio/OCEE/actions/runs/44748358687) attempt1与单独重试attempt2均`completed/failure`，但job `steps=[]`，`runner_id=0`，运行器根本未分配、没有Rust或测试日志；[同提交 S40 #44748358559](https://github.com/iFrankStudio/OCEE/actions/runs/44748358559)和[全workspace #44748358715](https://github.com/iFrankStudio/OCEE/actions/runs/44748358715)完全相同，不能归因于S41代码，也不能认为S41通过。私人仓库账单配额/组织Actions权限/托管runner供给需由GitHub UI进一步确认。没有可用本地cargo/rustc或外网依赖下载，现阶段不能做原生替代验证。
- 代码静态复核后改为 `key.starts_with(*prefix)` 与 `if_true.as_str()` 以明确Rust借用类型；提交使用`[skip ci]`停止对无效Actions运行器继续产生失败流水。**S41状态=未验/不记突破；G0 GO；G1/G2 NO-GO，E001未预注册。**待runner恢复后手动S41定向→S40回归→必要时workspace，不应所有历史study对每次小修自动运行。下一阶段科学目标转为请求ID绑定的故障证据与因果派生对象淘汰，减少actor级事件和人工目录先验。


### 2026-11-20 — CI预算治理，完整保留可复核性，不绕过S41未验状态
- 新增 [CI-REPLAY-POLICY.md](./CI-REPLAY-POLICY.md) 并将历史 S22–S40 定向工作流的 main push 触发改为手动`workflow_dispatch`，早期S4/S6原本已为手动；保留**当前S41推送定向触发**和**完整workspace在PR上自动检查、main上手动触发**。旧的Cargo执行命令/测试数据和GitHub Actions原生历史完全保留。动机：过去更改同一共享world-kernel/src/lib.rs或organism-seeds/src/lib.rs触发十多个原生作业，浪费私有仓库运行额度且导致早期长任务被自动取消。
- [S41 #44748358687](https://github.com/iFrankStudio/OCEE/actions/runs/44748358687) 两个attempt仍`runner_id=0, steps=[]`；[S40 #44748358559](https://github.com/iFrankStudio/OCEE/actions/runs/44748358559)及[全workspace #44748358715](https://github.com/iFrankStudio/OCEE/actions/runs/44748358715)同样未执行任何步骤。仓库owner需确认Actions预算/账户设置或托管运行器调度情况；没有可用编译/运行时不得谎报任何S41本地/原生成功。治理修改使用`[skip ci]`提交避免继续触发被账户环境阻断的jobs。**S41仍处于工程候选而非科学突破，G0 GO/G1/G2 NO-GO、E001未预注册。**
### 2026-11-20 — S42A：旧drop→最新受保护请求ID 的因果门（待原生验证）
- [S42A开发记录](./experiments/E001/DEV-S42A-REQUEST-CAUSAL-FENCE.md) · [内核实现及反例](./crates/world-kernel/src/lib.rs) · [手动CI](./.github/workflows/s42a-request-fence.yml)。从S40/S41代码静态审查识别 actor 级历史丢弃会误导当前在途请求恢复的风险。新`last_guarded_send` 随成功栅栏 SEND 持久化，`FenceStatus` 和 `RestartRoot` 都必须与最新真实消息ID、当前公开task hash匹配。
- 已提交一项内核负面对照（两条错峰受保护SEND、早丢一条、最新仍在途时拒绝旧drop触发重启；最新也丢弃时才返回可恢复状态），**尚无cargo/rustc及GitHub原生结果**。暂不修改原有S40/S41模型、不假报B计费结果。恢复runner后优先 S42A→S41→S40 定向，必要时workspace；G0 GO，G1/G2 NO-GO，正式E001未预注册。

### 2026-11-21 — S42B：活动P2 VM原生请求ID绑定的工程候选，CI未验
- [研究与因果判据](./experiments/E001/DEV-S42B-VM-REQUEST-RECEIPT.md) · [新活动P2专家](./crates/organism-seeds/src/s42b_request_receipt.rs) · [内核调用/VM ABI](./crates/world-kernel/src/lib.rs) · [单独手动CI](./.github/workflows/s42b-request-receipt.yml)。新`FenceStatusFor/RestartRootFor`接收完整u64请求ID，而非S42A最新请求猜测；VM在实际phase1 REMOTE_EVAL SEND成功后自费保存真实8字节回执，在Empty时读回同ID并用于故障查询/一次恢复。
- 新增**待运行**两个关键测试：大于u32范围的两个受保护在途请求错峰丢弃；真实P2活动VM发送、任务漂移、同ID状态查询及重启、重新学习和B仅结算一次。内核拒绝其他受保护请求仍在途或已有B>0服务事件时恢复（当前单个体E001安全优先、可能牺牲活性）。旧S40/S41个体保持独立、未改其程序图。
- 本轮无法原生运行，不得声称S42B测试成功或E001科学突破；恢复runner后按S42B→S42A→S41/S40→必要时workspace验证。G0 GO，G1/G2 NO-GO，E001仍未预注册。

### 2026-11-21 — S42B 静态代码复核更正，原生验证仍阻断
- 检查新增 `Action` 枚举后旧 S28 诊断的穷举匹配，补上两个新的 ID 绑定分支，避免 Rust E0004 编译失败。修正 S42B VM 回归断言：重启会清理**旧**请求ID，但第二轮正确新 SEND 会重新在工作区保存**新**请求ID；不得错误期待最终工作对象不存在。错峰双SEND单测在第二条刚到达之前不再把其ID误列为伪造负例，防止测试动作自身推动世界tick造成预期错位。
- 这三项均为静态推导的测试/诊断修正，不是 Rust 本地编译或远程 CI 结果。下一步必须以原生执行结果判断 S42B 是否实际实现了 request-id 因果恢复。

### 2026-11-21 — S43A 请求结果状态及已收回复因果认证（工程候选，未原生验证）
- [S43A记录](./experiments/E001/DEV-S43A-REQUEST-OUTCOME-AND-REPLY.md) · [内核与两项最小测试](./crates/world-kernel/src/lib.rs) · [手动定向CI](./.github/workflows/s43a-request-outcome.yml)。新增受费且带实际SEND审计资格验证的 `RequestOutcomeFor`（Unknown/InFlight/Dropped/HandledNoB/CommittedB）以及 `ConfirmResponseFor`（要求真实C→A服务消息和已发生的Receive全文一致），不自动重放、不放宽权限。
- 关键待证伪：合法P2计算即使B20已扣费而回复还在途，也不能视为任务未执行；只有真实Receive后才能认证该回复；正常协议诊断B0不是栅栏丢弃。u64消息ID保留，不等于协议内部u32工作ID。因果验证只在最近4096条审计窗口中寻找所有权/交付证据，过旧证据fail closed。
- **没有cargo/rustc可用，也没有S41–S43A原生成功证据；不声明新生命或E001确认结果。** 先执行S43A窄CI→S42B→S42A→S41/S40，再在有需要时workspace。G0 GO、G1/G2 NO-GO、E001未预注册。

### 2026-11-21 — S43A 静态复核：同协议ID不同消息ID的交叉回复负例
- 在已提交的S43A窄测试中追加**同一项测试内的关键对照**：合法P2 RemoteEval采用相同协议层u32工作ID重复SEND，内核分配不同u64消息ID，服务去重不再次扣B；把第一条实际收到的有效回复用于第二条请求的 `ConfirmResponseFor` 必须Conflict。防止把同一协议工作ID误当全局传输请求身份。将服务事件/栅栏丢弃证据扫描限制为最近4096条，以使单次查询成本有界；过旧状态降级Unknown/Conflict。
- 这仍是待原生验收代码；不得称跨网络 exactly-once、内核/VM单测通过或正式E001结果。验证状态/科学门槛不变。

### 2026-11-21 — S43B 活动VM在解析ACK/RESULT前做真SEND→RECEIVE因果确认（静态候选）
- [研究和反例](./experiments/E001/DEV-S43B-ACTIVE-RECEIPT-CORRELATION.md) · [P2活动个体](./crates/organism-seeds/src/s43b_active_receipts.rs) · [手动窄CI](./.github/workflows/s43b-active-receipts.yml)。每个phase1 ACK / phase2 RESULT候选 SEND 的真实u64回执均由VM写入 `/self/work/s43b-message-<phase><opcode>`，接收后主动读回并调用 `ConfirmResponseFor`，只有服务因果对应的实际Receive消息可进入旧协议解析。
- 不提高128个VM区块上限：新增5个受费确认节点，移除S43B新图中不再可达的4个S40/S41旧式角色级恢复节点；S40/S41/S42B旧图不改。3项**待运行**对照：健康P2 ACK+RESULT，真实任务漂移后新B请求再确认，注入未由服务产生的伪回复则终止而不写结果。任何未验收代码不记科学突破。
- S43B仍受研究者预定phase/工具/可用世界API限制，非通用异步网络主体；本环境无Rust工具链且GitHub runner最近未分配，待runner恢复后S43B→S43A→S42B→S41/S40窄回归。G0 GO、G1/G2 NO-GO，E001仍未校准/预注册。

### 2026-11-21 — S43A.2 同ID不同内容的伪造RECEIVE负例，待原生验证
- 对 S43A/S43B 的源证据依赖做静态安全审查：原先的回复认证缺少服务生成正文 hash，若向A邮箱注入同 response ID、改过正文的假消息再真的 RECEIVE，旧证据链接可误通过。服务事务现在随 `ServiceEvent` 一并存完整C→A Message哈希，`ConfirmResponseFor`要求内容指纹及到期交付，旧快照缺hash则fail-closed。
- 强化原S43A窄测试：向邮箱插入同ID修改正文，确实由Receive交付后认证仍Conflict；随后真服务消息仍可Ok，B费用与快照保持可追溯。此项为**待跑的回归断言**，不宣称已获得结果。

### 2026-11-21 — S43B 独立原生验证触发探针
- 对 S43B P2专家继承路径做静态图构造计数：S21角色压缩后、S22/S23、S31/S32、S40/S41、S42B及S43B共约120个命名区块（含删除4个不可达块后），低于VM的128硬限制；**这是静态推演，非Program.validate或cargo结果**。S43B原生真实图字节大小及运行时间仍需CI。
- 因现有连接不能主动运行workflow_dispatch、容器无cargo且无法访问私有GitHub，启用`s43b-active-receipts.yml`对**工作流自身修改**的单路径main push触发以尝试一次定向原生验证。此规则不会重复启动历史S22–S40，亦不会因S43B普通源码push反复运行。结果必须从实际Actions/job steps确认，不允许按代码已提交推断实验通过。

### 2026-11-21 — S43B 原生runner实际探针返回阻断（非Rust失败）
- [Run #44852637401](https://github.com/iFrankStudio/OCEE/actions/runs/44852637401) 由 `main` 提交 `876e687` 成功触发S43B单项workflow，`completed/failure`，job ID `136687346985`，`steps=[]`，未实际执行 checkout/rust-toolchain/cargo。没有证明S43B编译或VM通过/失败；不能写成科学验收。
- 为免浪费私有仓库Actions预算，已将S43B workflow恢复为仅手动调度。保持S40已验证4/4基线以及S41–S43B待验证闸门，G0 GO G1/G2 NO-GO，正式E001未预注册。应由有管理员权限的仓库/组织所有者检查Actions运行器、账单和策略后再按需手动运行最窄测试。

### 2026-11-21 — 仓库Public后原生Rust验证恢复：S43A 2/2、S43B 3/3、S41 4/4、S42B 1/1
- [S43B通过 Run #44860253323](https://github.com/iFrankStudio/OCEE/actions/runs/44860253323) @ `cf2217b`，3项独立活动VM测试步骤全部成功，覆盖健康 ACK/RESULT 绑定、实际任务漂移重启后新消息关联、无服务来源伪造消息拒绝。
- [S43A通过 Run #44860455869](https://github.com/iFrankStudio/OCEE/actions/runs/44860455869) @ `8805bea`，两项世界内核的执行状态/回复因果及栅栏丢弃与B0服务诊断负对照均通过。
- [S41/S42B混合 Run #44860113013](https://github.com/iFrankStudio/OCEE/actions/runs/44860113013) @ `ba0c664`：S41 4项+S42B 1项实际通过，另3项当时未修正的S43B失败，**整体Run失败**；S43B随后已被独立Run验收。此前 Runner 缺失、`E0716`、VM非法operand和参数类型错误均已留下可溯源失败证据。
- 仓库现为Public，GitHub-hosted Rust job正常运行。已撤销为验证而开启的S43A/S43B自动push触发，回归手动workflow_dispatch。仍需S42A专门回归及必要的workspace确认，不把局部验收声称全仓验收；科学闸门仍G0 GO，G1/G2 NO-GO，正式E001未预注册。

### 2026-11-21 — S42A 原生2/2通过，workspace全量原生测试已启动
- [S42A Run #44861540477](https://github.com/iFrankStudio/OCEE/actions/runs/44861540477) @ `db000dc0`，本次两个原生测试步骤 success，均各1项用例通过。S42A请求栅栏因果/缺失公共任务拒绝获得实际Rust验证。
- [Workspace Run #44861571121](https://github.com/iFrankStudio/OCEE/actions/runs/44861571121) @ `145a3034`，提交时已成功进入Rust `cargo test --workspace -- --show-output` 执行；尚未观测到最终结果，不能声称全仓通过。恢复S42A为手动workflow_dispatch，workspace为手动+PR触发。
- S43A原生2/2与S43B原生3/3的既有证据不变。科学阶段仍G0 GO、G1/G2 NO-GO、E001未校准/预注册。

### 2026-11-21 — S44 状态驱动的安全等待/恢复工程候选
- [设计、负例与界限](./experiments/E001/DEV-S44-CAUSAL-WAIT-AND-RETRY.md) · [活动P2 VM](./crates/organism-seeds/src/s44_outcome_wait.rs) · [原生3项定向CI](./.github/workflows/s44-uncertain-outcome.yml)。基于S43B真实SEND u64回执，Empty后查询S43A的RequestOutcomeFor：InFlight/CommittedB/HandledNoB只等待，Dropped只经核验的RestartRootFor，Unknown fail-closed，不盲目重做已可能消耗B的请求。
- 此为**尚未通过原生CI的研究候选**，不能与 S43A 2/2、S43B 3/3、S42A 2/2 的已验证证据混淆；完整workspace最终结论亦待其独立Run确定。维持G0 GO、G1/G2 NO-GO。

### 2026-11-21 — S44 原生通过3/3；S23全局高延迟负例保留
- [S44 Run #44862964346](https://github.com/iFrankStudio/OCEE/actions/runs/44862964346) @ `fed97e8c` completed/success，三个独立原生Rust用例全过：真实B请求+ACK定向延迟时InFlight/CommittedB均不盲重试、真实栅栏drop才触发一次root重学习、保存u64证据遭破坏时Unknown fail-closed。S44的Program.validate在测试内真实通过。
- [先前失败 #44862804649](https://github.com/iFrankStudio/OCEE/actions/runs/44862804649)：全局6-step网络延迟时虽B24一次、CommitB在途且无错误重启，但旧S23 RESULT操作码扫描耗尽其有界Receive预算，个体最终U32(0)。这是尚未修复的活性局限，不属于S44安全门通过范围。
- [S41回归 #44862672828](https://github.com/iFrankStudio/OCEE/actions/runs/44862672828) @ `b9934ae` 也独立completed/success，但共享lib模块改动不应重复触发昂贵的旧研究。因此S41和S44都改为仅手动workflow_dispatch并用`[skip ci]`封存。
- [完整workspace旧SHA Run #44861571121](https://github.com/iFrankStudio/OCEE/actions/runs/44861571121) 仍按其独立运行记录结论判断；其SHA先于S44，不代表S44全仓集成通过。科学G0 GO、G1/G2 NO-GO，E001未预注册。

### 2026-11-21 — 全量CI终态核验与S44负例边界（无新功能验收）
- 独立读取 [workspace #44861571121](https://github.com/iFrankStudio/OCEE/actions/runs/44861571121) 的运行及jobs记录：旧源码 `145a30348a67e8072a0d1ed09d776f35eeb54e09`，最终 `completed/cancelled`；`kernel` job同为 `completed/cancelled`，其中 `Run world and VM invariant tests` 步骤 `cancelled`，后续S8成本报告/manifest步骤 `skipped`。**这不是测试通过，也不能推断测试断言失败**；不需要为取得“绿色”而重复启动旧SHA全量CI。
- S44 [定向原生3/3 #44862964346](https://github.com/iFrankStudio/OCEE/actions/runs/44862964346) 的成功仅覆盖该workflow所选用例及其源码 `fed97e8cbfccfcb5415d510a71fb2503189c31c6`，不构成完整workspace回归。全局6-step延迟负对照 [#44862804649](https://github.com/iFrankStudio/OCEE/actions/runs/44862804649) 仍为真实红例：B已执行且仅扣24、CommittedB可见，但S23 RESULT操作码扫描在有界等待内未取得结果。后续方案应将**操作码搜索预算**与**已发送请求的在途等待预算**区分，避免把延迟误判为错误opcode；仍须保持同一u64请求ID的真实RECEIVE/服务确认、Unknown不重试、CommittedB不重发、有限A gas及独立低延迟/高延迟/伪回复负对照。这里只是从已有红例推导的待验证假说，**没有新Rust代码或CI验收**。
- 本轮仅完成CI终态审计，不调整G0 GO、G1/G2 NO-GO及E001未校准/未预注册状态。

### 2026-11-21 — 双重有界的Phase2 RESULT等待（S45候选，待原生验证）
- 根据最新 [VM循环作用域静态分析](./experiments/E001/VM-BOUNDED-LOOP-COUNTER-SCOPE.md) 和S44全局6-step真实失败，新增带局部scope和硬全局上限的 `RepeatScoped` VM终结子、继承S44的P2有限相位分派与 [S45定向原生验证](./.github/workflows/s45-scoped-result-wait.yml)。独立单opcode 6次Empty，总896次，上限持久化且不新增重发权限。
- 需以本次原生CI验收VM多scope+Yield+快照、P2全程6-step真实C/B及ACK/RESULT确认、原2-step正常对照。此处尚未有S45通过证据；全量workspace旧运行仍cancelled，G0/G1/G2及E001以实时检查点为准。

### 2026-11-21 — S45从全局6步延迟红例到原生3/3通过（本轮工程实证）
- **[原生 GitHub Actions #44905343381](https://github.com/iFrankStudio/OCEE/actions/runs/44905343381)** @ `e66a691b0c00e8a47ce1f561dc2820c8139905d1`，completed/success：VM `RepeatScoped`双层有限计数+Yield/快照（1/1），S45真实P2**从头到尾延迟6步**ACK/RESULT因果认证和真实B24**仅一次**（1/1，61.51s），原2步对照正确fold32/B24（1/1，31.88s）。S44原全局6步红例是B24而U32(0)；S45固定112候选搜索下将phase2每opcode等待限制6且全部opcode总额896，实得U32(1)并通过真实Request ID确认与快照校验。
- 失败保留：[S45 #44905202275](https://github.com/iFrankStudio/OCEE/actions/runs/44905202275)旧S30诊断未穷尽VM新终结子Rust E0004；[S45 #44905261924](https://github.com/iFrankStudio/OCEE/actions/runs/44905261924)编译成功但错误假设S22 Empty直接走S23，实际S41/S42B已转入phase判定；两者已修复并通过最终窄CI。
- [研究全文](./experiments/E001/DEV-S45-SCOPED-RESULT-LIVENESS.md) · [S45 P2活动程序](./crates/organism-seeds/src/s45_scoped_result_wait.rs) · [VM scoped有界器](./crates/organism-vm/src/lib.rs)。临时工作流push路径恢复仅`workflow_dispatch`。这修复的是封闭确定性C环境的已知高延迟活性问题，不等于多网络条件下无错恢复或开放演化。全仓旧Run #44861571121 cancelled；研究G0 GO、G1/G2 NO-GO；E001未校准/未预注册。

### 2026-11-21 — S45组合故障交叉因果安全实验候选
- [研究协议](./experiments/E001/DEV-S45-COMBINED-FAULTS.md) · [两项原生目标CI](./.github/workflows/s45-concurrent-faults.yml) · [追加的真实活动P2测试](./crates/organism-seeds/tests/s44_uncertain_outcome.rs)。此前 S45 全局高延迟（6步）3/3已过，但未测试真实任务版本漂移与延迟同时发生的恢复，也未在S45的新Phase2等待路径中注入无服务来源伪回复。
- 本轮仅增加两项有明确资源/因果不变量的交叉对照：旧请求真TaskFenceDrop/B0、新请求仅B24一次并真ACK，及RESULT阶段伪回执应ConfirmResponseFor Conflict/无结果成功。**尚未获取本轮原生CI结论**，后续只能按实际Run更新，不能预先记通过；科学闸门不变。

### 2026-11-21 — S45 组合故障真实原生验证2/2完成
- [Actions #44909113918](https://github.com/iFrankStudio/OCEE/actions/runs/44909113918) @ `dc7022ccdfb98120b85b613ba7c4e066fcc5c0dc`，**completed/success，原生2/2 PASS**：全局6步延迟下真实任务漂移触发旧RemoteEval的TaskFenceDrop/B0，活动个体按真实ID付费恢复一次并重新学习，新请求真实B24只执行一次、ACK经过内核确认、最终结果正确（57.90s）；全局6步延迟下Phase2 RESULT已真实发送后注入无服务来源伪消息，虽Receive Ok但ConfirmResponseFor Conflict，无伪成功/B重复/无因重启（51.78s）。逐项真实tick/audit/账本及快照验证。
- [详细研究](./experiments/E001/DEV-S45-COMBINED-FAULTS.md) · [原生两项组合对照](./crates/organism-seeds/tests/s44_uncertain_outcome.rs) · [仅手动重放的CI](./.github/workflows/s45-concurrent-faults.yml)。保留之前S45 3/3原生基线；新证据仅为有限组合故障工程安全，不代表更广泛真实网络或自然开放演化。旧workspace仍取消、科学G0 GO、G1/G2 NO-GO、E001未预注册/校准。

### 2026-11-21 — S45超出已验收延迟窗口的单点探索（待原生结论）
- [研究与解释规则](./experiments/E001/DEV-S45-DELAY-BOUNDARY.md) · [原生单项工作流](./.github/workflows/s45-delay-boundary.yml)。既有S45 delay6成功及组合故障2/2成功均不代表delay8；新增全程delay8真实活动P2探针，打印最终U32结果、B账本、真实u64 SEND ID、VM作用域等待计数及重启记录，强制B<=24、RemoteEval<=1、无无因重启、总预算<=896、成功必须有真实fold32及B24、失败不得写成功结果。**CI尚未运行；测试绿色不自动等于delay8活性成功，必须读取日志的return字段。** 科学G0/G1/G2与E001状态不变。

### 2026-11-21 — S45 全程八步延迟新增原生正例
- [Actions #44911181744](https://github.com/iFrankStudio/OCEE/actions/runs/44911181744) @ `bb924f3bbb46cf4b163204a3992665c679b34d64` **completed/success，单项原生1/1 PASS（43.31s）**。日志明确 `delay=8 return=Some(U32(1)) b=24 remote_ids=[26] result_waits=162 restarts={}`；成功分支验证真实fold32、仅一次B及SEND、快照哈希一致。不同于仅安全测试绿色，本次确实有delay8活性成功证据。此前delay6及组合故障2/2证据保持独立。此固定P2开发seed单点不能外推为通用延迟保证/科学进化。详见[研究记录](./experiments/E001/DEV-S45-DELAY-BOUNDARY.md)。工作流恢复仅手动。旧workspace仍取消，科学G0 GO、G1/G2 NO-GO，E001未校准/预注册。

### 2026-11-21 — S45 新P2协议实例的配对消融（开发集、待原生验证）
- [预定的独立协议种子17/93、全程delay8、S44/S45配对四条轨迹](./experiments/E001/DEV-S45-PAIRED-PROTOCOL-SEED-ABLATION.md) · [真实VM测试](./crates/organism-seeds/tests/s44_uncertain_outcome.rs) · [定向CI](./.github/workflows/s45-protocol-seed-ablation.yml)。此前S45 delay8成功仅在协议seed42，无法直接外推；本试验固定工作任务、世界seed55、预算与服务语义，只改变未暴露给活动个体的协议种子，每组做S44/S45两臂。无论成功与否验收B不重复、真SEND不重发、真实来源确认、成果不伪造及快照；**CI绿色不等于四臂活性都成功，必须读取四行S45_PAIRED_ABLATION**。此条为提交前实验设计，尚未原生验证。

### 2026-11-21 — 冻结的P2协议种子配对消融：安全通过，任务0/4失败
- [真正原生Run #44916855269](https://github.com/iFrankStudio/OCEE/actions/runs/44916855269) @ `519f9ccb20d389400490eb6accee61b82397d1b9`，completed/success仅表示本轮预设的4条安全路径断言通过；seed17/93 × S44/S45旧图四臂实际都返回U32(0)、B0、没有真实REMOTE_EVAL；共享s23_receive_retry=128，s45_result_retry=0、step411，表明未到Phase2就耗尽Phase0等候预算。原生[实验记录及完整失败表](./experiments/E001/DEV-S45-PAIRED-PROTOCOL-SEED-ABLATION.md)。**绝不把安全绿色误报成活性成功。**
- 新设工程回归：仍固定协议seed17/93和全程delay8，只在S45 Phase0增加独立每opcode与总额双限等待；Phase1的真实request outcome/restart与Phase2安全确认不放宽，之后单独验收两种协议中的真B24一次、真实ACK/RESULT及快照。原旧配对运行SHA和阴性结论永久保留。本段新修复**尚未获得CI通过证据**。

### 2026-11-21 — S45 Phase0 REMOTE_EVAL探索修复，冻结失败样本回归2/2成功
- [既有配对阴性 #44916855269](https://github.com/iFrankStudio/OCEE/actions/runs/44916855269) @ `519f9ccb` 四臂安全通过但活性0/4（seed17/93每组S44、旧S45均B0，旧S23共享128预算提前耗尽），已不可逆归档。
- [**原生修复Run #44917248521**](https://github.com/iFrankStudio/OCEE/actions/runs/44917248521) @ `4cbf8743b63eff8de55ae1d8bdd3b872f9e11d8e`，**completed/success，1项Rust测试、内部两完整活动P2轨迹2/2真成功（407.03s）**：seed17全程delay8结果U32(1)、真实B24一次、真实RemoteEval ID120，Phase0/Phase2分别354/312次scoped wait、tick2140；seed93同为U32(1)/B24一次、ID108、阶段等待318/360、tick2198。均具真正ACK与RESULT `ConfirmResponseFor` 通过、结果fold32正确、无无因Root重启、快照一致。
- 新S45 Phase0 REMOTE_EVAL opcode探索和Phase2 RESULT各采用**单候选6、每阶段总896**的局部+全局双限；Phase1受限真实ID恢复及Phase3不变。修复前失败样本和成功后的目标相同，属**开发集定向修复**而非新独立确认集；先前42协议seed既有基线不等于当前修订SHA全回归。旧workspace取消，不得说全仓通过；G0 GO、G1/G2 NO-GO，E001未校准/未预注册。
- [完整负/正因果证据及研究限制](./experiments/E001/DEV-S45-PAIRED-PROTOCOL-SEED-ABLATION.md)。两次临时workflow push触发恢复仅手动，以免历史实验自动反复运行。

### 2026-11-22 — 执行前冻结的四P2协议实例保留集：原生运行候选
- 自动研究 [预先冻结的保留集协议](./experiments/E001/DEV-P2-FROZEN-HOLDOUT-PROTOCOL.md) 定义全程 delay8 / P2 protocol seed **311, 587, 941, 2027**，被测活动P2代码为 S45 Phase0+Phase2 已验证版 `4cbf8743`（当前 `main` 无该程序的语义更改）。本次只添加 [一项被显式忽略的原生保留集测试](./crates/organism-seeds/tests/s44_uncertain_outcome.rs) 及 [四种子定向矩阵CI](./.github/workflows/p2-frozen-holdout.yml)，不改变个体程序、协议、世界或资源。每实例独立运行原14k tick夹具、持续delay8、日志输出实际成功/B/ACK/RESULT/等待/VM gas及真实opcode作用域边界；失败仍纳入分母4。
- 初始状态：**仅代码已提交，CI及保留集活性未验收**；测试绿仅意味无B双扣/盲重试、所有明确安全断言成立，不等于四条均成功。与正式E001预注册无关，G0 GO、G1/G2 NO-GO。

### 2026-11-22 — 执行前冻结的4个 P2 开发保留实例原生 4/4 成功
- [唯一首次原生 Run #44983342484](https://github.com/iFrankStudio/OCEE/actions/runs/44983342484) @ `a8c812a0f59ca5477e00c36080c2019e2fddbbf7`，**completed/success，四个隔离的Rust matrix jobs全部通过**，真实任务 `U32(1)`、独立fold32、B24一次、真正C-SEND/ACK与RESULT的u64 ID认证、快照及双层界限均通过。预先冻结协议seed 311/587/941/2027，全部在原本固定 `message_delay=8` 下执行，且被测P2程序哈希四组一致 `0f9163bdff40a2ac5cbee63586353ea394f639d6dc7cc190ac34c811634c518e`。
- 四组的Phase0/Phase2等待分别300/366、612/294、348/342、348/6；VM gas分别75,721、94,248、76,969、36,466；A动作gas分别11,552、11,925、11,554、10,156；完成tick分别2167、2811、2223、1103。成功为**4/4预固定开发保留实例**，不是正式E001确认性集/真正未知语法自主发现。原研究夹具 `a_gas_per_epoch=100000` 明显高于E001初始12000、后4000的资源门槛，仍没有A/B/C/D对照或净成本优势证据；G0 GO、G1/G2 NO-GO，E001未校准/未预注册。完整逐实例原生记录见[保留集报告](./experiments/E001/DEV-P2-FROZEN-HOLDOUT-PROTOCOL.md)，工作流恢复仅手动。

### 2026-11-22 — P2 富裕资源成功后，开始E001式A gas压力校准（未验收）
- [预先固定的资源校准协议](./experiments/E001/DEV-P2-A-GAS-PRESSURE.md) 与 [两seed 2027/587 原生定向工作流](./.github/workflows/p2-resource-pressure.yml)。此前[保留集4/4](https://github.com/iFrankStudio/OCEE/actions/runs/44983342484)使用高达A 100000 gas/epoch的装载与运行夹具，不能替代E001初始12000、8192步下调4000/epoch的验证。新测试保持原S45协议/世界语义不变，明确将研究者已计费装载记为**特权free-genesis反事实**，运行前重置A账户12000、计划真实资源变更事件。选两组已见开发seed（省资源2027、高成本587），最大原14000个自主tick，区分任务成功、失败和未完成删失；完整账本与真实B24/ACK/RESULT判据不放宽。**待原生CI日志，不得提前记通过或失败**。

### 2026-11-22 — 补充收费基因组初始条件验证（seed2027，待原生验收）
- 前述免费装载压力试验已显示seed2027在12000 A gas/epoch的初始资源下活动运行成功，初始基因组装载曾实际消耗8818 A gas但由实验夹具重置余额；现追加[真实收费genesis对照](./experiments/E001/DEV-P2-A-GAS-PRESSURE.md)与[独立定向CI](./.github/workflows/p2-charged-genesis.yml)，强制先用12000真实额度写四次研究者提供的程序并验算余额，再保留余量让个体活动且不特权补偿，8192步按E001要求降4000。seed仍为已公开2027，非新独立保留样本；不改变S45活动程序及其hash。**候选待CI验证，不预报活性结论。**

### 2026-11-22 — A 12k→4k真实epoch资源压力：原生2/2成功（带free-genesis重要限制）
- [Run #44987651536](https://github.com/iFrankStudio/OCEE/actions/runs/44987651536) @ `2452464e299b1c39192ef911a47e3a4dcd3cf7c3` completed/success，2独立原生jobs：协议seed2027在每epoch 12000下第3103步真`U32(1)`/B24一次、VM35934+A操作1318、Gas拒绝2022次；seed587第8217步真`U32(1)`/B24一次、VM94045+A操作3097、Gas拒绝5419次，正规资源事件已在8192将其A额度降为4000/epoch。均通过真实u64 ID ACK/RESULT确认、fold32及快照校验，无盲重试/未授权重启。
- **此两实例预置基因组由研究者在100000 gas配置下写入花费8818，然后强制把账户补回12000**；这是一种有利的free-genesis反事实，不能报告成完整E001资源符合。单独的实际12k计费基因组对照（seed2027）仍需读取本独立[charged-genesis workflow](./.github/workflows/p2-charged-genesis.yml)结论。此阶段没有证明4000/epoch下长期自治、工具原创或本地基线净收益。完整记录见[资源压力报告](./experiments/E001/DEV-P2-A-GAS-PRESSURE.md)。研究门槛G0 GO、G1/G2 NO-GO，E001未预注册/校准。

### 2026-11-22 — 初始12000 A gas真正计费的P2启动与任务原生成功
- [Run #44988134196](https://github.com/iFrankStudio/OCEE/actions/runs/44988134196) @ `2c50ac7490370f30571d44997e8f28fed052731d` completed/success，原生1/1：协议seed2027，4次初始genome Write在真实首epoch12000额度内扣8818（初始余3182，无特权补满），后续VM36310+A动作1329、Gas拒绝2266次，**step3351真U32(1)和B24只一次**，ACK/RESULT实际u64 ID118/120内核认证、fold32与快照一致，无无因重启。合计总A gas46457，跨epoch正常补充。
- 收费启动实例早于8192结束，故尚未直接验证“收费启动 + 4000额度降额”同时成立；另一个free-genesis seed587确实降额后完成，须严格分开。无自主生成协议/净资源优势证据。 [协议与原生账本](./experiments/E001/DEV-P2-A-GAS-PRESSURE.md)，定向CI仅手动，科学G0 GO、G1/G2 NO-GO。

### 2026-11-22 — P2收费genesis+真实4k降额组合，固定高成本seed587（待原生验证）
- 已核对实时main `9e811559949d28c96897147bbec9bca713acfe4a` 和最新Actions，没有新的手动/自动研究覆盖旧证据。此前[free-genesis下的587降额正例](https://github.com/iFrankStudio/OCEE/actions/runs/44987651536)与[2027收费genesis正例](https://github.com/iFrankStudio/OCEE/actions/runs/44988134196)分别通过，不能合并推断同一活动轨迹成功。新增[单一轨迹组合实验协议](./experiments/E001/DEV-P2-CHARGED-GENESIS-DOWNSHIFT.md)及[只针对一项原生CI](./.github/workflows/p2-charged-genesis-downshift.yml)，固定已见开发P2 seed587、消息delay8、初始A12000真实Write扣费且不补余额、step8192降至4000；额外要求降额后**实际VM gas扣费>0**、最终任务U32(1)、真实ACK+RESULT认证及B24一次。原有专用活动程序未更改；观察期不延长。**待真正原生CI结果，不预报成功**；科学G0 GO、G1/G2 NO-GO，E001未预注册/校准。

### 2026-11-22 — P2收费genesis与真实A降额4k同轨迹原生验收成功
- [GitHub Actions #44989858138](https://github.com/iFrankStudio/OCEE/actions/runs/44989858138) @ `11f5845cb7f76ff3b43e7f7b31248a24e61291d0`，**completed/success；Rust单项1/1 PASS（564.36s）**。固定已见开发seed587/P2延迟8，研究者提供S45初始程序在真实A12000额度内四次Write支付8818、首次活动余额3182，未补回；step8192正规降至4000，降额后仍产生**267个有效VM tick、真实VM gas9911**；在step10303个体真U32(1)/B24只一次，RemoteEval u64 ID206，ACK和RESULT真正因果确认、fold32和快照一致，Gas拒绝7500，Invalid/Denied为0。活动A VM94229+world动作3105，加启动8818后真实总A**106152 gas**，原S45活动程序hash未变化。此前free-genesis seed587成功step8217，两实验严格保留不同初始化条件；这次补齐此前无法合并的单条组合轨迹。
- [原生研究报告与明确局限](./experiments/E001/DEV-P2-CHARGED-GENESIS-DOWNSHIFT.md)。仅证实固定研究者专用程序在分epoch紧资源中有状态继续执行并完成一次真实任务；**不证明**个体自行优化A资源、创造协议适配器、跨家族演化或远程任务净成本优势；G0 GO、G1/G2 NO-GO，E001未校准/正式预注册。完成的一次性workflow恢复仅手动。

### 2026-11-22 — 同一真实四U32操作数的A本地/远程计算边际成本对照（待原生CI）
- 实时main `9632f8dbdc2d89d0ccb007f78e5e0d3de878404e` 已复核，无新自动提交。上一轮已证明seed587收费genesis及4k降额能完成；**远程计算是否节省A资源仍缺证据**。启动[研究协议](./experiments/E001/DEV-P2-MATCHED-LOCAL-FOLD-COST.md)及[唯一原生目标CI](./.github/workflows/p2-matched-local-fold-counterfactual.yml)：固定已见开发P2 seed2027，收费12000 A启动，原S45先自行生成16字节真实REMOTE_VALUES并成功B24/因果认证，之后仅由外部研究夹具从真实完成快照分叉，使用研究者额外编制的通用纯VM本地FOLD32，原样接收同一16字节，原生真实A注册/EXEC扣费、对比逐项真实A/B和C通信成本。本地纯实现使用ByteAt/Add/Div及绕转/溢出控制，禁止宿主提前提供结果，不能算个体自行发现本地算法或完整同起点A-only对照。**现仅代码和实验设计提交，未取得新CI结论，G0 GO、G1/G2 NO-GO**。

### 2026-11-22 — 首次原生同一真实操作数A本地FOLD32边际成本反事实通过
- 原始[Run #44991946675](https://github.com/iFrankStudio/OCEE/actions/runs/44991946675) @ `000ad9a`因测试误取ActionResult.result.gas_charged而编译E0609失败，未运行；仅修正4处字段访问。修复[**原生Run #44992095414**](https://github.com/iFrankStudio/OCEE/actions/runs/44992095414) @ `718c710ac55c5afbc48a2f96cd4ab7a5f3f9d36b` **completed/success，1/1 Rust测试（264.37s）**，原S45收费genesis开发seed2027真实完成任务step3351、B24一次、真正ACK/RESULT请求ID认证，原A总gas**46457**（VM36310+动作10147，含genesis8818）、发往C真实成功SEND**61次**、A→C计费带宽**1726**。从原world快照分支，由研究者另写的**通用纯VM本地FOLD32**真实扣A安装**346**、对原root实际生成的同四个LE U32执行仅**165**，合计**511 A gas**；结果都为`1415179467`，与B及独立fold32一致，原世界/个体决策不变、无第二次B。
- 这是首次真正量化**已知四个U32时末端本地FOLD32便宜**；不是同起点等任务对照，不能直接将46457/511相减称远端净亏损：远端路径还承担公共任务学习、工具验证与未知第四操作数生成，本地算法由研究者预制且只在事后得到实际操作数。**尚无远程执行对A的净节约证据，必须开展A-only与A+B完整等资源配对基线及长期复用控制**。详见[实验原生账本与局限](./experiments/E001/DEV-P2-MATCHED-LOCAL-FOLD-COST.md)，旧workspace CI取消状态不变，科学G0 GO、G1/G2 NO-GO，E001未校准/正式预注册。定向工作流恢复只手动。

### 2026-11-22 — 原始快照同起点完整任务A-only/A+B配对（开发集，待原生验证）
- 已复核main `54db435fc98bbb9202814a3e96de4a5d0ce6952e` 与最新原生记录。此前同真实U32末端反事实不具完整任务等起点。新增[执行前固定的配对方案](./experiments/E001/DEV-P2-EQUAL-ORIGIN-LOCAL-REMOTE-PAIR.md)和[只跑两臂的原生CI](./.github/workflows/p2-equal-origin-route-pair.yml)：固定已见P2 seed2027，12k gas/epoch收费装载P1、原S45 P2、完全相同根程序及本地FOLD32，分叉前hash完全一致；两臂只由**研究者**等额Write一字节L/R。共同root完成原公共学习、probe、第四操作数后，选择真实本地VM Exec与付费Write，或原S45真实远程B24+ACK/RESULT，严格对齐任务、观察/资源/初始能力；各记录A/B/C gas和完成/失败分母。**路由是研究者预设，不是自主资源优化；CI未验收，不能声称成果**。

### 2026-11-22 — 同起点路由配对首轮L失败与后置有界gas重试（开发修复）
- [首轮Run #44994405005](https://github.com/iFrankStudio/OCEE/actions/runs/44994405005) @ `bb401d1a989802fb9579b01f9ddb292882909aa2` 的L臂真U32(0)失败于step1025，B0/C-SEND0/无成功本地Exec、gas拒绝981；R当时仍运行，严禁报完整任务2/2。初步诊断是隔离Exec在低可用A余额下返回Gas，原本root分支非Ok即失败，区别于连续VM的跨epoch进度保存；现开发后置修复仅允许本地Exec遇真Gas时经`RepeatBounded limit2048`重新Read输入并付费重试，其他非Ok仍fail-closed，不增加A额度/改变seed/路由/算法工具；追加实际Exec-Gas日志用于验证机制。**这不是重新预注册，需重新原生验证两臂。** 详见[完整研究记录](./experiments/E001/DEV-P2-EQUAL-ORIGIN-LOCAL-REMOTE-PAIR.md)。

### 2026-11-22 — 等起点A-only/A+B原生首次1/2，L失败保留，修复版待CI
- [Run #44994405005](https://github.com/iFrankStudio/OCEE/actions/runs/44994405005) @ `bb401d1a` 已完成：**R=U32(1)真成功**，A47350/B24/C-SEND61/step3383；**L=U32(0)失败**，A12015/B0/C0/step1025、无成功local Exec。两臂初始world state hash和共同root/初始化费用完全一致；无法把L未完成成本当成功。已独立提交`2bd9fc9`仅真Exec Gas有界重试修复，新一轮两臂完整复测尚未验证；必须保留首次红例。

### 2026-11-22 — 更正L失败根因：原root的P1切换被误截获；撤回无证据gas重试
- [二次 CI #44994931874](https://github.com/iFrankStudio/OCEE/actions/runs/44994931874) @ `9d9afdc8` 两臂均在启动前`InvalidProgram("repeat limit exceeds 256")`，无完整任务执行，原红例保留。进一步读源码发现`switch_active`最初用于P1学习路由，原研究者`L`在操作数还未产生时截获P1，直接Read缺失对象而导致U32(0)。先前将L失败猜测为isolated Exec缺gas**缺乏依据**，现从开发图中撤销后置非法/未经证实的有界Gas重试。修复后只在switch目标hash**确为S45 P2**时接受本地或远端选择，P1切换完全原样，仍为相同收费genesis/程序内容hash两臂、同任务同资源；待第三版原生配对验证。完整负例及纠偏见[实验记录](./experiments/E001/DEV-P2-EQUAL-ORIGIN-LOCAL-REMOTE-PAIR.md)。

### 2026-11-22 — 首次等起点完整任务本地/远程配对2/2真成功：远程多花35123 A gas
- [第三版原生CI Run #44995228916](https://github.com/iFrankStudio/OCEE/actions/runs/44995228916) @ `46a47f91f76d738c9cfbc69bcd93d5aa54b84bcc`，**completed/success，2个隔离原生Rust jobs均通过**。两臂同一初始world `state_hash=85e36d4f8db82e7b2c4ebf829a93bcdd3d3390441ca6012a21e47e969fe1ed23`，同一活动root hash `94fbac40626e577c63e7dc310cd0ea57be1468d9259d95b359017c7b4307d840`、共同真实genesis**9462 A gas**（余额2538）、route设置均3 A gas。同seed2027/全程delay8/12kA分epoch/相同真实任务与工具，研究者L/R一字节路由只在真正P2 handover起作用；此前错误地截获P1导致L失败，现保留首次[1/2红例 #44994405005](https://github.com/iFrankStudio/OCEE/actions/runs/44994405005)及VM循环limit错误[0/2红例 #44994931874](https://github.com/iFrankStudio/OCEE/actions/runs/44994931874)，不写成原次成功。
- **L本地**：完整任务真U32(1)、独立fold32 **1415179467**、B0/C-SEND0、单次付费本地纯VM Exec165、总A**12236**（VM2368+动作9868）、Gas拒绝981、step1027；**R远程**：同真实结果及真B24一次，RemoteEval ID118与ACK/RESULT真实确认，C-SEND61/带宽1726，总A**47359**（VM36555+动作10804）、Gas拒绝2279、step3383。双方快照正确、无非法VM/错误Root重启。**同起点真实资源净差`R-L=35123 A gas`，远程另耗B/通信且多2356步**；此开发任务远程路径不具A gas净优势。
- **重要限制**：L/R由研究者控制路由，公有P2完整专家和本地FOLD32均由研究者预装且两臂等额付费，不是个体自行比较工具或原生演化；尚未建立正式E001 A/B/C/D机制、跨任务/跨家族与长生命周期可继承结果。完整[方案、三次原生运行、负例与成本账本](./experiments/E001/DEV-P2-EQUAL-ORIGIN-LOCAL-REMOTE-PAIR.md)；本实验workflow恢复仅手动。科学G0 GO、G1/G2 NO-GO。

### 2026-11-22 — 实际本地EXEC反馈选择本地/远端路径（开发候选，待原生证据）
- [实验冻结协议](./experiments/E001/DEV-P2-FEEDBACK-AVAILABILITY-ROUTE.md) · [独立原生两臂工作流](./.github/workflows/p2-feedback-route.yml) · [活动VM路由根与测试](./crates/organism-seeds/tests/s44_uncertain_outcome.rs)。前一等起点实测本地12236 A、远程47359 A的负收益已封存；它的单字节路由仍是研究者指定。本次撤销同源根的研究者L/R读取，在真实P2 handover后由同一VM先付费尝试本地Exec，返回Ok则直接保存正确本地FOLD32；仅真实NotFound才切换已有P2专家、付费完成B24及双请求因果确认，Gas/其它错误不盲切。两个不同初始能力条件PRESENT/ABSENT分开审计，**不误称同起点成本净对照**；S45协议/资源和任务保持开发seed2027/延迟8/12000 A/14k tick。
- 新工作流仅对其自身文件的这次push安排一次定向matrix；**当前尚无原生CI结果，不宣称成功、更不宣称已学会净收益最优策略**。存在明显后续反例：本地工具存在但返回错误值时，单凭Exec Ok不能判断语义正确。科学G0 GO、G1/G2 NO-GO；E001未预注册。
- [首次原生 #44997491803](https://github.com/iFrankStudio/OCEE/actions/runs/44997491803) @ 84c3f7f：ABSENT 原生 1/1 真成功，Exec NotFound=1、B24一次、真正u64 ACK/RESULT认证、结果1415179467、A47017、step3366；PRESENT 同源码原生 0/1，step1025 提前U32(0)、没有本地Exec、B0、A12033；整体Run为红色，不能记整项通过。两臂初始能力不同，不能做同起点成本差解释。先对PRESENT追加只在失败时打印末段真实审计（不改变VM、样本、预算），单独调试，不预设是缺Gas还是阶段早切；后续两臂必须一起复验。
- [PRESENT诊断 Run #44998024761](https://github.com/iFrankStudio/OCEE/actions/runs/44998024761) @ 9cef866 真实失败：step40 Read工作输入成功；step41 实际本地Exec扣85 A gas后Code::Gas，因活动root未区分临时Gas与真正NotFound而回到failed，长达约983个无A额度tick后step1024 U32(0)结束。不能说完全没有尝试Exec。本次只新增8次硬上限的付费Gas-only本地重试路径；NotFound唯一触发B回退，不提高A gas额度、tick上限或改变已有种子。两臂已恢复在同一新源码下的独立完整CI矩阵，结果待验证；首次红例永久保存。科学G0 GO、G1/G2 NO-GO，E001未预注册。

### 2026-11-22 — 真实 Exec 反馈路由最终 2/2 原生通过，失败链保留
- [最终原生 Run #44998219987](https://github.com/iFrankStudio/OCEE/actions/runs/44998219987) @ afcfa56b37d8950629ac9e7617a4855ecef5bae9，completed/success，PRESENT/ABSENT 独立Rust各1/1通过。相同活动根 d1de40feea835290be333152058b2de07193f0fb0844e47b6869cf7604e1e7db 无人工L/R读路由，而在P2 handover后按真实本地Exec反馈作选择：PRESENT 真正本地 Exec Ok1次、A12318、B0/C0、step1028；ABSENT 真正 Exec NotFound1次才转远端、A47055、B24一次、C SEND61、step3368，RemoteEval ID118与真实Result ID120由ConfirmResponseFor确认。两条件均U32(1)、实际结果1415179467，独立fold32及世界快照一致。
- 首次两臂 Run #44997491803 中PRESENT失败，后续 [诊断 #44997843628](https://github.com/iFrankStudio/OCEE/actions/runs/44997843628) 和 [精准动作诊断 #44998024761](https://github.com/iFrankStudio/OCEE/actions/runs/44998024761) 均红色，揭示step41真实Exec扣85 A gas后Code::Gas，旧根误当硬故障；后置只增加8次RepeatBounded的Gas-only重新Read+Exec，真实NotFound仍是唯一远端回退，其他错误fail-closed。无额度补偿、无更换样本或延长窗口。完整研究见[实验报告](./experiments/E001/DEV-P2-FEEDBACK-AVAILABILITY-ROUTE.md)；专用workflow已设为仅手动。
- 两条件的初始本地工具可用性与genesis A真实费率不同，**不构成同起点净成本对照**；能力边界仅为研究者写定的条件策略在活动VM中依据真实付费反馈执行，而非成本学习、原创算法或开放式演化。G0 GO，G1/G2 NO-GO，正式E001仍未校准/预注册。

### 2026-11-22 — 真正语义Ok假成功三臂原生3/3：验证了可执行并不等于正确
- [原生 Run #45002443707](https://github.com/iFrankStudio/OCEE/actions/runs/45002443707) @ 8fb5f1b297498224dc6439912f8f3a36ffcd7412，completed/success，VALID/ABSENT/CORRUPT 三独立原生Rust jobs 3/3通过，且三者相同真实活动root hash 6bdcf2bd416d52f7e922d8de383dbd63b31de9eda7e30aa5d9d499435714410d，种子2027/delay8/12000 A gas/epoch/14k ticks不变。VALID 真 Exec Ok=1415179467、真实写入正确值并U32(1)、总A12286、B0/C0、step1027；ABSENT 真 Exec NotFound=1并真实远程B24一次、C SEND61、确认RemoteEval u64 ID118及Result ID120、总A47025、step3368且正确；**CORRUPT 的合法收费纯VM在同一真实四U32输入返回Exec Ok=1415179468，但独立真值1415179467，活动根仍写入错误值并返回U32(1)**，总A12291、B0/C0、step1027。三臂均无非法越权/无因恢复，snapshot一致。**绿灯证明负反例存在，绝不代表个体已修复语义错误**。
- [实验冻结、原始程序与结果](./experiments/E001/DEV-P2-SEMANTIC-OK-COUNTEREXAMPLE.md)；该反例只表明研究者预置的可用性回退缺少自有独立语义证据，不自动意味着开放式学习失败，也不能以不同初始能力3臂推断等起点净效用。下一闸门必须以实际、合法、计费的独立证据建立拒绝假成功的机制，验证冲突时不保存且不盲切B；所有正确参考值只在研究者测试端出现，不可由VM特权读取。一次性定向CI恢复仅手动，G0 GO、G1/G2 NO-GO、E001未预注册。

### 2026-11-22 — 双纯VM工具付费语义冲突原生4/4通过：能拒绝分歧但仍无法识别共错
- [GitHub Actions #45019867266](https://github.com/iFrankStudio/OCEE/actions/runs/45019867266) @ 00d69284ef427397476edeb54c34e75901631640，completed/success，四个独立原生Rust jobs AGREE_VALID/DISAGREE/AGREE_WRONG/UNKNOWN 4/4。四臂真正相同活动根hash 2a4f5080f1fffc38452889cdc413466b61421829fbd33f9fd85134e215de1f14；根自行经付费Read读两个研究者注册的程序hash、对自行产生的同16字节输入分别真Exec、在内部比对收到的payload。AGREE_VALID 两次Exec Ok=1415179467且正确落盘U32(1)，总A13446、B0/C0；**DISAGREE** 两次Exec Ok=1415179467与1415179468，根返回U32(0)、无成功落盘，总A13405，确证**实际冲突拒绝**；**AGREE_WRONG** 两个不同合法程序均真Exec Ok=1415179468，独立正确值1415179467，根仍保存错误值并U32(1)、总A13458，确证**两份一致不能证明正确**；**UNKNOWN** 首次Exec Ok=1415179467、第二次真NotFound，返回U32(0)、无落盘、总A12525。四臂均真实genesis收费、无B/C、快照一致、无非法重启；UNKNOWN不计入双Exec Ok分母。不同候选安装与genesis成本不能作等起点净资源因果对照。
- [完整冻结协议、四臂原生证据与局限](./experiments/E001/DEV-P2-METERED-DISAGREEMENT-PROTOCOL.md)；一次性CI恢复workflow_dispatch仅手动。这里仅有研究者预设的有界冲突检测机制，未获得独立的语义真值、学习路径选择或原生演化证据。下一门槛为合法、付费、非共同错误来源的任务证据，及不一致/不确定时的保守处理。G0 GO，G1/G2 NO-GO，正式E001仍未校准预注册。

### 2026-11-22 — 收费零输入任务见证：能识别恒定共错但漏掉零点保真的共错；原生4/4
- [第一次#45039624800](https://github.com/iFrankStudio/OCEE/actions/runs/45039624800) @7149890首次四臂均因 Program.validate("block count") 在世界任务之前0/4失败。严格MAX_BLOCKS=128不动，仅从root entry遍历保留各种明确控制流边上的可达块（旧P2路径不可达图裁除），二次[真实Run #45039761198](https://github.com/iFrankStudio/OCEE/actions/runs/45039761198) @4bfa7d2 completed/success，4个独立jobs 4/4。四臂root共同hash 2245a5d2a9bce52ab64ad4219e3803f8f98f17ccddc5a40172356cad829a6a84；世界seed55、P2 seed2027、delay8、A12000/epoch收费genesis、14k自主tick上限不变，非原生oracle。
- VALID 真任务Exec两次1415179467、主工具第三次有偿零输入Exec=0（165 gas），正确写入结果、U32(1)、总A13675；COMMON_PLUS_ONE真任务双Exec都1415179468、额外零输入Exec真返回1（167gas）与根内公开FOLD32零约束冲突，返回U32(0)、**没有结果落盘**、总A13640；**COMMON_DOUBLE**双不同程序真Exec都2830358934且全零Exec真=0（166gas），根依然落盘这个错误值并U32(1)，总A13684，证明单零约束无法保证全部输入正确；DISAGREE原任务两Exec分别1415179467和1415179468，根直接返回U32(0)，没有额外零输入Exec、无落盘、总A13380。四臂B0/C-SEND0、无非法重启、snapshot一致，独立真实fold32=1415179467只在测试审计出现。
- [完整预冻结协议及包括首次失败的原生账本](./experiments/E001/DEV-P2-METERED-ZERO-ANCHOR-WITNESS.md)；零规则由研究者预设、工具也由研究者安装，这是有界任务见证不是自主发现/学习或一般正确性证明。不同初始安装计费不可作等起点资源净收益因果结论，下一阶段需更强独立任务可验证证据及跨任务应对共错；独立CI恢复为workflow_dispatch仅手动。G0 GO，G1/G2 NO-GO，正式E001未校准预注册。

### 2026-11-23 — P2付费任务变形见证最终4/4真成功，同时保留共享错误可绕过反例
- 新[执行前冻结的变形验证协议及完整红绿账本](./experiments/E001/DEV-P2-METERED-METAMORPHIC-CHECK.md)：活动VM先真Exec两套独立实现的工具、比较实际四U32同任务结果，再真Exec已知全零见证；全部都通过时由根付费Read自己实际生成的16字节工作输入、以通用ReplaceByte/Xor翻转byte0最低位并付费Exec主工具，利用公开FOLD32任务关系 `F(x0 xor 1,x1,x2,x3) = F(x) ± 1` 验证任务具体输出变化，宿主正确值仅用于外部断言。根与工具由研究者预设，非自主学会验证。
- 首次[原生#45099227328](https://github.com/iFrankStudio/OCEE/actions/runs/45099227328)四臂因严格VM MAX_BLOCKS=128越界而在世界执行前失败；压缩相邻反馈块后[原生#45099406062](https://github.com/iFrankStudio/OCEE/actions/runs/45099406062) **3/4**，COMMON_PLUS_ONE未生成P2输入；单臂[实证诊断#45099701115](https://github.com/iFrankStudio/OCEE/actions/runs/45099701115)定位step27公共P1 held-out Write实际Code::Gas，活动根提前U32(0)。不得掩盖两次红例。后置仅将13个新研究P2见证块及对应边映射为更短的一对一ID，节约真实有偿genesis A31 gas，不提高12k/epoch或更换开发seed2027。
- [最终原生Run #45100054624](https://github.com/iFrankStudio/OCEE/actions/runs/45100054624) @e25b0b2 completed/success，**四个隔离Rust jobs均1/1通过，同一活动根hash b2fb6aac8dbffbac702afbfe3682ffef044b221e07853d6912d4253cf4e1703f**。VALID原值1415179467、变形值1415179466，U32(1)正确落盘，总A13887；COMMON_DOUBLE两工具实际都算2830358934、零点真0、变形真2830358932（差-2而规则应-1），拒绝成功且无落盘，总A13849；COMMON_PLUS_ONE原任务双错1415179468但零输入真Exec=1，直接拒绝且不运行变形Exec，总A13621；**INVARIANT_FAULT**两套不同合法代码皆在FOLD结果加原真实后15字节和1415，零输入0且变形差-1均通过，仍把错误1415180882落盘并U32(1)、总A14272。这4组B gas=0/C成功SEND=0，Invalid/Denied=0，无非法重启且快照一致。
- 科学解释：检测共同乘二错误是值得保留的窄成功，但可绕过的INVARIANT_FAULT是同等重要的反例；不能将双程序一致+零点+一次局部变形误称一般正确性。A跨臂初始装载不同，未做同起点效益识别。该实验是见过seed2027的开发集，不是E001正式预注册，科学闸门G0 GO，G1/G2 NO-GO。定向CI恢复为仅workflow_dispatch手动。

### 2026-11-23 — 付费双坐标任务变形首次原生4/4，检出旧共错但仍存在共同不变错误
- 已从[执行前冻结的双坐标协议](./experiments/E001/DEV-P2-TWO-COORDINATE-METAMORPHIC-PROTOCOL.md)落实实际Rust活动VM程序，延续原S45 P1/P2完整任务和P2开发seed2027、A12000/epoch、14k自主tick，严格不改VM MAX_BLOCKS128。新根从126块添2块，复用活动VM已真实付费Read的原16B输入及原程序hash，在两次原任务Exec一致、零输入正确、byte0 xor1输出变化正确后，自行对**原输入**的byte1 xor1再有偿Exec并比对期望±256。宿主独立正确值绝未给根。
- [原生GitHub Actions #45102384014](https://github.com/iFrankStudio/OCEE/actions/runs/45102384014) @1c30c9701da0e41535bcea04d5450052ab855bec **completed/success，独立Rust四臂4/4**，共同活动根hash 89676236d878c0591aa5c0c3979c2e81b16145d841c4d7fb30bcaa7f8d090329。VALID任务值1415179467、byte0变形1415179466、byte1变形1415179211，正确落盘U32(1)、总A14212；前次漏检INVARIANT_FAULT原错误1415180882、byte1变化从预期-256实测**-257**，新根拒绝落盘U32(0)、总A14596；COMMON_DOUBLE原错误2830358934、byte0变化-2违反±1，在第二坐标前拒绝U32(0)、总A13951；新RESIDUAL_FAULT两个不同工具均对正确值加 `sum(input[2..15])=1190`，零输入0，两个坐标变形分别满足-1/-256，依然**错误落盘1415180657并U32(1)**，总A14672。所有条件B0/C成功SEND0、Invalid/Denied0、无非法重启、世界快照一致；额外byte1单次Exec为165/212/215 A，完整成本另含VM和初始化。首轮原生即4/4，无红例被后验修复掩盖。
- [完整方案、四臂真实资源账本及明确误判负对照](./experiments/E001/DEV-P2-TWO-COORDINATE-METAMORPHIC-PROTOCOL.md)；不同初始安装导致总A非等起点净效用，研究者预设两程序和两约束，不是自主证明/学习/演化。固定有限样例继续有新的共错不变性，下一研究不宜只无限堆叠固定坐标，应探索可付费验证的结构化任务证书与证据选择。定向CI恢复workflow_dispatch仅手动；G0 GO、G1/G2 NO-GO，E001仍未正式预注册。


### 2026-11-23 — 从有限变形到完整付费校验器：三个独立原生组证据与信任边界（VALID待收尾）
- 已冻结[完整结果复核与恶意校验器协议](./experiments/E001/DEV-P2-PAID-EXACT-CHECKER-PROTOCOL.md)，代码基于原真实S45 P1/P2任务：研究者安装的独立纯VM校验器接收活动根自己Read的原16字节操作数及原候选真正Exec返回的U32 claim（总20B），重新按FOLD32规格计算完整值与claim比较，纯VM U32(1)才允许根真实Write成功工件；根没有宿主真值或特权oracle。
- [首次CI Run #45103750605](https://github.com/iFrankStudio/OCEE/actions/runs/45103750605) @1eeb01c：当前实际3臂completed/success，**VALID仍in_progress，不能写4/4**。共同根hash 03a0e04248abef1c4520c5633f4c84daef456e783d5a29e71952c7d7e41fe0d6：PLUS_ONE原真Exec1415179468、checker真Exec U32(0)/334 A，根拒绝无落盘，总A13497；RESIDUAL_FAULT上轮两个固定坐标漏掉的真错1415180657、checker U32(0)/334 A，拒绝落盘总A13646；**COMPROMISED_CHECKER**同样错1415180657却以研究者恶意注册的常真checker真实Exec U32(1)/2 A，活动根误写错误工件并返回U32(1)，总A12660。各组B0/C成功SEND0、无Invalid/Denied、快照恢复一致。
- 这不证明通用无信任正确性：研究者作者提供的真实完整计算checker才是独立可信规格假设；变更checkerhash就能伪造成功。独立工具部署/认证、低成本结构化证书与真正自主证据选择尚未实现。定向工作流恢复仅手动；完整初始Run仍须待VALID完成后复核，其他3组不能代替。G0 GO、G1/G2 NO-GO、E001未预注册。


### 2026-11-23 — 内容哈希锁定完整校验器原生4/4，通过注册表替换和缺失负对照
- 研究者[冻结协议](./experiments/E001/DEV-P2-PINNED-EXACT-CHECKER-PROTOCOL.md)使用现有世界内容寻址不可覆盖的 `/self/versions/<hash>`，在活动根中把研究者授权的 FOLD32 完整参考校验器hash当作常量，普通VM `JsonFieldBytes` + `EqBytes` 对经付费 `Read("/self/work/exact-fold-checker")` 取得的hash做身份闸门。没有新增VM特权或不收费oracle；后续有偿Exec完整20B(16B真工作输入+4B被声称U32结果)按真实Code::Ok/U32(1)才写盘。研究者预置pin并非个体自主发现的信任链。
- [原生GitHub Actions #45105207973](https://github.com/iFrankStudio/OCEE/actions/runs/45105207973) @9d65a0413 **completed/success，独立4/4**；相同活动根hash `adf01a69992dcb09628d50ff8c7999adbe345f85ae667b17e38a7d4d1b55c368`，trusted hash `333b612826ee95417a647184dae6d3bc243b5da34d2ed1758f05361c05ef7590`。VALID_PIN原任务真1415179467，付费可信checker Ok/U32(1) 334 A，正确落盘，总A13551；WRONG_PIN实际错1415180657，checker Ok/U32(0)/334 A，拒绝落盘，总A13657；REGISTRY_SWAP真原错1415180657、registry读到恶意hash，根在**未Exec任何checker**前拒绝，另研究者在安装前尝试向可信哈希写恶意Program真返回Code::InvalidCode、不创建/覆写可信对象，总A13225；MISSING_PIN registry正确，但真实可信Exec返回NotFound，拒绝落盘总A12584。均B0/C成功Send0、VM自主Invalid/Denied0、无非法重启、快照一致。主根和任务、资源上限、S45链均未放松；跨臂安装计费不同不可误称等起点效益。
- 前一无pin的[原生#45103750605](https://github.com/iFrankStudio/OCEE/actions/runs/45103750605)当时仍有旧VALID job in_progress；本次新root VALID_PIN的成功**不代替/追认那组旧job**。实验结果证明研究者预设可信锚可防注册表指向错误校验器，但不证明初始锚来源、可安全轮换或任意代码正确性；不是自发学习/开放式演化。定向CI恢复仅workflow_dispatch，G0 GO、G1/G2 NO-GO，正式E001尚未预注册/校准。完整账本见[实验报告](./experiments/E001/DEV-P2-PINNED-EXACT-CHECKER-PROTOCOL.md)。


### 2026-11-23 — S29错误继承工具可通过实际VM反馈修复并CAS有界采用；最终原生4/4
- 在早期S29[三算子迁移实验](./experiments/E001/DEV-S29-EVIDENCE-SELECTED-OPERATORS.md)中，不匹配语义的来源P1 `XorFold` 工具带到P2 `Len` 世界后仅能检测错误、U32(0)中止。这次[运行前冻结与原生账本](./experiments/E001/DEV-S29-REPAIR-ON-INHERITED-MISMATCH.md)新增**真实活动VM**的有限修复：第一次带标签真实EXEC错→原有三个通用原语基于公开两例重新选择→真实`RewriteOp`生成新内容寻址程序→正常付费Write→`expected_version=0`注册表CAS→修复后独立一次性Exec校验两例+独立heldout→完整P2工作继续。严格VM无界环校验不动，未扩大A12k/epoch、世界seed129、P2协议seed42或16,384 tick界；源工具真正来自P1个体学习与保存，非研究者凭空复制正确答案。
- [最终原生#45107700394](https://github.com/iFrankStudio/OCEE/actions/runs/45107700394) @e7f602399 **completed/success，隔离Rust 4/4**，四组共同root hash `f8e5d42b1d4286395b6b3453801264c819e4b97d53159d8c24cb04e480b4b5fe`。FRESH无工具→新Len版本Write1、真实Exec3、heldout5正确、B20/P2真值1156832460、A23984/step1183；MATCHED实际导入Len→目标运行期**版本Write0**、真实Exec3、heldout5/B20、A23816/step1183；WRONG导入XorFold→首样例Exec43对标签3失败→重选Len，版本CAS0 Ok更新至version1、三次新Exec、heldout5/B20、A24064/step2051；STALE人为先将旧注册提升version1→旧Exec43错、研究者预置的CAS0 Conflict且None重试Conflict、无heldout/B0/错误落盘、U32(0)，A6653/step14。四组无自主Invalid/Denied与无授权重启，snapshot一致；STALE留下已生成但未采用的有偿不可变Len版本，**不是原子回滚**。
- [首次原生#45107240682](https://github.com/iFrankStudio/OCEE/actions/runs/45107240682)静态VM发现无界环而4/4失败（无世界结果）；改成单向修复后复核[#45107371940](https://github.com/iFrankStudio/OCEE/actions/runs/45107371940) 三组语义成功但MATCHED因测试误算导入Write而3/4，最后仅修正审计时间边界后4/4。三次红绿链永久归档，专用workflow恢复workflow_dispatch。该开发证明**已知有限候选中真实反馈选择、修复、CAS采用的活动VM机制**，不代表新算法发明或开放式演化；不同安装条件不构成等起点总A效益比较。G0 GO、G1/G2 NO-GO，正式E001未校准/预注册。此前无pin完整版check的[#45103750605](https://github.com/iFrankStudio/OCEE/actions/runs/45103750605)最终`completed/cancelled`、旧VALID job cancelled，不能写4/4。


### 2026-11-23 — S29第二公开样例首例碰撞仍可原生有偿修复；首次4/4
- 已按[预冻结协议和完整审计](./experiments/E001/DEV-S29-SECOND-LABEL-REPAIR-PROTOCOL.md)实现第二标签失配的真实活动VM恢复路径：此前S29只在第一公开标签不符时重选工具；本轮研究者有偿安装一份合法`ByteAt(Bytes,1)`对抗工具，目标P2 world129第一例[32,3,8]输出3与`Len=3`相同，第二例[18,2,7,4]真输出2≠公开标签4。活动VM付费Exec发现第二例不匹配后**有偿重新Read公开双标签**、再依据原有三原语选择Len、真实RewriteOp生成可执行、不可变版本有偿安装、CAS0采用、另一路单向真实Exec检验两标签及heldout并继续真实P2 B操作。未更改VM静态无界环和MAX_BLOCKS、A每epoch12k、world129/P2协议42、16k tick限；跨臂根一致。
- [首次native#45115636978](https://github.com/iFrankStudio/OCEE/actions/runs/45115636978) @1e8305195 **completed/success，独立Rust4/4**，活动root hash `d2774946312c4aa9cc10132c754d58aab6dfde9be15320d14e02f9b4e57667a1`。MATCHED真P1 Len工具原地复用，目标自主新增版本0次、heldout5/B20、A23997、step2051；FIRST_MISMATCH真P1 XorFold首Exec43≠3、自动修复、CAS0 Ok/新Len三次Exec/heldout5/B20、A24248、step2051；**SECOND_MISMATCH**研究者对抗工具真Exec首3=3次2≠4、额外真Read公共双例、自动修复新Len三次Exec、CAS0 Ok/v1、heldout5、P2真实计算结果1156832460正确、B20、**A24436、step2056**；SECOND_STALE同样真实首3次2错误后虽重选和Write新Len，却因研究者预提升registry version1导致CAS0 Conflict+None Conflict，禁止采用/heldout/B，A7022、step16。四组InvalidCode/Denied0、无越权重启、snapshot一致。跨组安装不同不代表等起点因果效益。
- 现行世界`/self/tools`注册Write需先有已安装的`/self/versions/<hash>`，而VM/world只公开**单键CAS Write、无Delete/原子双键提交**；SECOND_STALE留下有偿创建的未注册新Len版本，安全拒绝不等于事务回滚。下一次应把**可信工具替换的原子性、失败清理/垃圾回收**作为独立系统机制问题，不要仅堆叠校验样例。研究者仍提供有限三个原语、修复图和公开标签；无新原语发明、无确认性独立30seed配对/开放式演化，G0 GO/G1/G2 NO-GO，E001未校准预注册。该定向workflow仅手动。


### 2026-11-23 — 原子Program版本安装+registry CAS发布，首次原生四臂4/4，不再残留失败孤儿
- 针对[S29第2公开样例修复](./experiments/E001/DEV-S29-SECOND-LABEL-REPAIR-PROTOCOL.md)旧`先Write版本→再CAS注册表`导致STALE安装孤儿的新缺口，先冻结[原子PublishTool通用世界能力协议](./experiments/E001/DEV-S29-ATOMIC-TOOL-PUBLISH-PROTOCOL.md)，再增加类型化`WorldCall::PublishTool`和世界`Action::PublishTool`：仅允许`/self/tools/`，从活动VM输入的规范Program字节计算实际内容hash，先检查owner/registry CAS/旧版本字节/存储Quota/整笔A gas，成功后一次原子插入不可变版本并更新同actor工具注册指针，失败则无版本/registry/持久容量变化；旧Write权限不变，纯Exec不允许副作用。
- [首次 GitHub Actions #45119834399](https://github.com/iFrankStudio/OCEE/actions/runs/45119834399) @51da269d945e **completed/success，4独立Rust jobs 4/4，无原生红例**。三条实际S29活动VM共有root hash `1c5b7f793793c4b8bdf4b45861f5a13fa5ce50fb438c8e0aa04b7352a3e19a8b`，世界P2 world129/protocol42、A12k/epoch/最多16k ticks与有限三原语搜索仍不变。REPAIR原ByteAt首例3与Len一致、第二例2≠4，根实际重新读取双公开标签、生成Len规范程序、一次PublishTool CAS0 Ok收25 A、新工具Exec3,4,5、heldout5与真正B20/P2结果1156832460正确；A总**24400**、step2056。FRESH注册不存在、首CAS0 Conflict后None重试一次PublishTool Ok收25 A、正确Exec3,4,5、heldout5/B20/P2真结果，A总**24129**、step2051。**STALE**旧registry被研究者提前提升v1后双样例反证，真实CAS0 Conflict和None Conflict均0 A，**新Len程序版本对象不存在，registry未变，actor.storage_used=101294B前后不变**；无heldout/B0，A总**6931**、step15。三组无自主Invalid/Denied/越权重启、snapshot一致。
- 第四内核FAILURES组：真实Denied（非法路径）、InvalidCode（非规范程序）、Quota（空间不足）、Gas（本epoch不足）、Conflict（陈旧CAS）均不创建版本或registry、`storage_used`不变；已拥有的同hash不可变版本CAS复用收6 A，无重复版本对象。完整成本、真实Exec及CAS轨迹见[报告](./experiments/E001/DEV-S29-ATOMIC-TOOL-PUBLISH-PROTOCOL.md)。
- 新增`PublishTool`是**研究者定义的第7类通用工具发布能力**（相对于原E001六原语），改变了世界动作与初始能力集；正式E001前要版本化冻结内核、成本和允许动作，不能因新能力成功归因于自主演化。失败事务无孤儿≠旧合法版本被自动GC，亦非全世界tick可撤销；成功后安全回收、引用计数/异步版本持有与多epoch容量压力是后续独立研究。科学闸门 G0 GO/G1/G2 NO-GO，正式E001未预注册/校准，专用CI恢复workflow_dispatch-only。


### 2026-11-23 — 原生受费保守程序版本回收：最终4/4，有真实高成本反例
- 先冻结[DEV-S29-METERED-VERSION-RECLAIM-PROTOCOL.md](./experiments/E001/DEV-S29-METERED-VERSION-RECLAIM-PROTOCOL.md)，再增加受限`Action/WorldCall::ReclaimVersion`：仅能申请本actor规范内容寻址不可变程序，保护任何活动/continuation程序和永久初始恢复根，其他对象key/raw64字节hash及未完成消息有可见引用则保守Conflict，成功时先付费扫描+删除（不碰registry/active），原子移除版本与精准减少storage_used。维护根自己的活动Program字节码被排除原始文本扫描，以便在普通VM常量中明确指定回收hash；因此它仍不是对任意嵌入/转码引用的通用无误删GC。
- 最终定向 [#45121695557](https://github.com/iFrankStudio/OCEE/actions/runs/45121695557) @bd8a1bf46 **completed/success，独立Rust四臂4/4**。POST_REPAIR真实S29原子修复+完成P2任务B20后，研究者付费安装并按`/sys/active-version`真实版本CAS切换维护根，活动VM真回收旧ByteAt工具，删对象/释放**279B**、实际动作收费**1646 A**，新Len仍真Exec正确、B不重复、total A26143、step2063、快照一致；REGISTERED有第二`/self/tools`引用时首次Conflict，合法CAS移走引用后第二次才Ok释放279B；ROOTS保护active/initial/maintenance根，坏hash InvalidCode、缺失NotFound；OPAQUE_REF保护显式pin/真实在途消息hash，移走引用后回收277B收费9 A，Gas不足拒绝不部分删除。
- 红例不可抹除：[首次#45121234194](https://github.com/iFrankStudio/OCEE/actions/runs/45121234194) 4/4编译失败（Rust变量标识符撇号），[第二次#45121401232](https://github.com/iFrankStudio/OCEE/actions/runs/45121401232) 2/4成功（两长组夹具错误假设/active仍v0，被真实CAS拒绝），后改为经付费世界`Read(/sys/active-version)`取版本再CAS，在未改回收语义/资源/seed前提下取得最终4/4。定向workflow恢复仅手动。
- **关键负面成本**：279B回收却耗1646 A，只能证明在保守引用扫描下有真实资源费用，不能推断净效用为正。维护启动由研究者选择，不是生命体产生自己的长期资源管理目标；第8类世界能力变更也不能跟原正式E001六原语公平混算。下一阶段应优先验证可信索引/租约降低扫描费的条件和过期索引反例。G0 GO、G1/G2 NO-GO，正式E001未预注册/校准。


### 2026-11-23 — 内核认证引用证书使同快照三程序回收收费下降65.9%，原生4/4
- 继上一真实S29回收279B需1646 A后，先冻结[认证索引协议](./experiments/E001/DEV-S29-CERTIFIED-REF-INDEX-PROTOCOL.md)，实现kernel-owned `VersionRefIndex`（快照持久化）、付费完整`WorldCall/Action::IndexVersionRefs`和只接收仍有效证明的`ReclaimIndexedVersion`。注册写/版本安装/SEND/RECEIVE/EXEC见证对象/环境任务发布/HELLO定期更新/在途消息交付/根重启均强制失效，root实时保护检查；缓存仅能在成功删除对象、没有新增引用的条件下保守复用。用户自写`/self/index`不被内核信任；原完整扫描GC保持。
- [最终首次完整native#45123571075](https://github.com/iFrankStudio/OCEE/actions/runs/45123571075) @382cde319 **completed/success，独立Rust4/4**。AMORTIZED在真实P2 world129/S29修复B20后，同快照同三版本`[279,276,279]B`（合计**834B**）配对对比：旧完整Reclaim×3真收费`[1657,1651,1646]`=**4954 A**，新活动维护VM真付费索引扫描105858B建证书**1667 A**、三次快速GC`[8,8,8]`A=**1691 A**，**GC世界动作收费节省3263 A（65.9%）**，两组删相同版本、存储与B20不重复、snapshot一致，新root U32(1)。注意这仅是同快照**世界动作费**对比；旧分支由外部驱动器发动作，没有配对的旧维护根VM费用，不能写成完整生命体净效用。
- STA​LE_WRITE真正Write新原始hash pin→旧证书失效、fast/legacy均Conflict，付费重建证书也识别活引用；STALE_MESSAGE真正SEND在途引用→失效、两者Conflict；GUARDS活动/初始root、坏hash、缺失、Gas不足都安全拒绝无删；三组原生均通过。
- [首次#45123180187](https://github.com/iFrankStudio/OCEE/actions/runs/45123180187) 3/4（AMORTIZED研究者夹具`ByteAt(0,0)`将Bytes当U32，VM`InvalidProgram`），仅纠正合法`ByteAt(0,1)`索引寄存器（索引值仍0），机制/阈值/seed未改，最终4/4。专用CI恢复workflow_dispatch-only。
- 这是研究者新增两项受限世界能力并定义有界维护程序，仍不意味着生命体自主GC、开放式演化；索引只跟踪当前已安装程序的**原始ASCII完整hash**，未知编码和外部异步保留引用不在安全证明中。后续关注整个维护VM净成本、证书失效频率及正式六原语能力边界。G0 GO，G1/G2 NO-GO，E001未校准/预注册。


### 2026-11-23 — 原生资源感知的活动VM决定是否GC：最终4/4，证明有限条件自我维护
- 先冻结[资源压力维护决策协议](./experiments/E001/DEV-S29-RESOURCE-PRESSURE-GC-POLICY-PROTOCOL.md) @`7bbfb45e`，发现旧Observe没有自身storage_used，因此新增仅限自身读、收费2 A的`Read("/sys/resource-status")`，精确返回`{used,capacity,a_available}`，U32溢出拒绝，既有世界操作能力不加任意信息Oracle。研究者向真实S29/P2运行完后世界有偿安装3个已有候选、切换到一个固定的有界维护根，**四组同一根 hash `29db9593562403a3d48f42ffc11f4e44861f4d8a223f982184a6672bed5ea8d1`**，VM用真实付费Read后评估空闲<约25%且剩余A≥2200则尝试一次Index+最多三次安全GC，否则返回U32(0)完全不索引；失败也不盲试后续。
- [原生#45127009918](https://github.com/iFrankStudio/OCEE/actions/runs/45127009918) @26b1675bb2f **completed/success，独立Rust4/4**。PRESSURE实际容量108805/已占108549、可用A11259，同根自主决定回收三旧程序`[279,276,279]`B，真实Index1667 A、GC8+8+8 A、退还**834B**、最后U32(1)、先前B20未重复；RELAXED容量217098/已占108549、A11259，同根不执行索引/GC、不删、返回U32(0)；LOW_GAS容量108805/已占108549、观察A1494，同根因预算不足也不扫描不删、U32(0)；INVALIDATED同高压力A、付费Index1667后有真实合法Write新增首hash pin64B使证书失效，下次GC真实Conflict且不删、停止两项后续，U32(0)，**浪费1667 A的负例**。四组真实S29修复P2/heldout正确、B20、新Len仍可Exec正确、快照一致。
- 首次[#45126843831](https://github.com/iFrankStudio/OCEE/actions/runs/45126843831) 4组均Rust E0502编译红（测试审计借用跨可变Exec），后仅移动额外Exec核验至审计引用使用完毕，无资源策略/研究协议改动；最终4/4。专项CI恢复仅workflow_dispatch。
- **重要研究边界**：研究者仍选择进入维护阶段、提供3候选及固定资源阈值，所以这是一种**基于真实环境状态的有限自我维护行为选择**，非形成内生目标、环境自行规划维护、完整算法自主演化；新元状态可见面也需正式E001版本化。索引遇环境写入真实有1667 A沉没成本，下一步应验证写频与证书存活概率/GC触发收益函数，而不是假设每次高压力索引都值得做。G0 GO/G1/G2 NO-GO，E001未预注册/校准。


### 2026-11-23 — 真实GC失败回执持久化，下一轮同根主动退避、再次恢复：首次native3/3
- 从最新main `7d89d001`及[资源压力策略](./experiments/E001/DEV-S29-RESOURCE-PRESSURE-GC-POLICY-PROTOCOL.md)继续，先冻结[失败记忆/退避研究协议](./experiments/E001/DEV-S29-PAID-PERSISTENT-MAINTENANCE-BACKOFF-PROTOCOL.md) @`1d677fab`，**完全复用既有VM WorldCall::Read/Write/IndexVersionRefs/ReclaimIndexedVersion**，无新增内核原语。研究者定义单步退避根 `33945b358e2a76d5c12c167bd2e6603a0b989ec012c1a77c4c74d790fe862af0`：首次实际GC Conflict后由活动VM付费普通Write自身`/self/maintenance/risk-s29`=`1`(CAS None→v0)，跨snapshot/研究者付费CAS重新激活同一根，下轮真Read1后自行跳过昂贵GC并CAS0写0→v1；第三轮从普通自有状态0恢复压力/预算决策，不修改现有引用索引安全与计费。
- [首次native#45130306800](https://github.com/iFrankStudio/OCEE/actions/runs/45130306800) @7379ef5817178 **completed/success，独立Rust三臂3/3，无本轮红例**。RACE_BACKOFF_RECOVER先真实P2 world129、protocol42、S29修复成功B20，再三次同根维护：实际研究者pin造成首次Index后GC Conflict、VM真实风险Write1(v0)；第二轮VM真Read并跳过Index/GC全部，Write0(v1)；研究者真实CAS把pin移走后第三轮再Index并真GC3×Ok，释放834B，实际存储`111449+64+1−834=110680 B`，世界step2083、B仍20、snapshot一致。PAIRED_REATTEMPT同完整snapshot第二轮带记忆根0 A索引扫描，而付费切换到固定压力根则索引收费**1786 A**、后续仍Conflict无版本删除；两程序根不同，不能把差值写成全历史VM成本。CORRUPT_MEMORY普通自身记忆非法单字节[2]使活动VM故障闭合、Index/GC0次/不删版本，B与snapshot仍正确。专用CI手动触发。
- 证明范围是**研究者预设的一次退避与跨运行真实失败记忆**；研究者仍负责每轮重新激活同根、候选版本与撤销pin，个体无自发重试唤醒、概率估计、阈值演化；历史多次失败CAS版本约束可能阻止进一步记忆更新。G0 GO/G1/G2 NO-GO，正式E001未预注册/校准。下轮优先研究移除研究者外部激活依赖，并固定写频/时间延迟作独立配对长期验证。


### 2026-11-23 — 同一个VM单次激活后有界连续维护，首次原生3/3；无新增内核能力
- [执行前冻结协议](./experiments/E001/DEV-S29-BOUNDED-SELF-RETRY-CONTINUATION-PROTOCOL.md) @038537695；复用现有原生`RepeatBounded(limit3)`和快照持久`Continuation.loop_counters`以及已收费Read/Write/Observe/Index/GC，不修改kernel/VM/actions/cost table。研究者仅一次付费激活维护根 `dfdd293c2602cc68c709d40dfa447e19620e314c8ed16114c771290e2a1114c0`，此后同根的失败/冷却经真实收费Observe在实际 `tick_autonomous` 中回到有界循环，**0次研究者/VM重新Write(/self/active)**。
- [首次原生#45141256292](https://github.com/iFrankStudio/OCEE/actions/runs/45141256292) @6468a250b6 **completed/success，独立Rust3/3、无本轮红例**。ONE_ACTIVATION_RELEASE真实世界S29 wrong ByteAt第二标签修复+P2正确B20之后，研究者有偿安装三旧合法版279+276+279B，首次活VM付费索引后真实pin造成GC Conflict、VM真实普通CAS记风险1(v0)；第二轮VM经Observe真实读1、跳过索引并写0(v1)，之后研究者真CAS移除pin（不是重启），第三轮同VM重新索引并GC三次Ok，回收**834B**，存储`112006+64+1−834=111237B`，累计VM A17806/action A10931/step2079（均含前置工作）、B仍20、snapshot一致。PIN_STAYS保持真实引用，索引两次、GC两次Conflict、无误删，第三轮旧固定CAS None遭v1 Conflict，静态三轮上限安全结束U32(0)（这是未解决重复失败记忆写的负结论）。SNAPSHOT_CONTINUATION在真实首次冲突风险Write1后复制完整world，包括活动vm_continuation和计数1，双方后续正常tick、真实移除pin并完成三次GC，最终**完整snapshot+audit逐字节相同**、计数3、B20，证明真正恢复而不是重装。
- **科学边界**：本轮仅排除维护**每轮外部重新激活**，外部驱动器仍逐tick推进世界，个体没产生维护目标/候选/新的阈值；没有跨整epoch或长期概率学习，固定退避1轮、最多3轮。正式E001六原语未被直接改动，但用到了先前研究扩展。G0 GO/G1/G2 NO-GO，E001未校准/预注册；下一步以长期资源轨迹、重复失败CAS版本处理和非预置候选为题，避免堆重复成功验证。


### 2026-11-23 — 实际自身CAS版本反馈使同根连续两次GC失败仍可记忆：原生首次4/4
- 先在[协议](./experiments/E001/DEV-S29-OWN-OBJECT-CAS-VERSION-RETRY-PROTOCOL.md) @11f45480 预冻结，再在`Action::Read`增加**有偿且仅限actor自有规范路径**`/sys/self-object/{relative}`，原子返回真实普通对象`{bytes:[u8],version:U32}`，收费`2+ceil(len/64)`A，原始`Read(/self/..)`保持原文不变，禁止跨actor/路径穿越/版本溢出。不增加世界动作类型、GC扫描和VM指令，但**改变了开发世界可见信息表**，正式E001需要另行冻结。
- 研究者提供同一固定有界维护根 `88af07516c0e101b260b7950f0e1de5afbfeb5e556952b4bd26b26a2be306815`，继承原来真实P2 world129/protocol42、S29 Len原子修复/heldout/B20、初次有偿安装三旧版和`/self/active`首次CAS启用；之后永不重启根，通过原有持久RepeatBounded(limit3)和有偿Observe续执行。真正GC Conflict后VM有偿重读自己的最新元数据，NotFound→Write风险1 CAS None，已存在→用刚读到的U32版本真正CAS写1；冷却读取b1则真CAS当前版本写b0，任何读/写版本错误都安全退出。
- [原生#45145971582](https://github.com/iFrankStudio/OCEE/actions/runs/45145971582) @b6682deab4 **completed/success，独立Rust四臂4/4，无本轮红例**。REPEAT_CONFLICT pin一直存在：Index2次、GC`Conflict,Conflict`，VM风险普通Write`1(v0)→0(v1)→1(v2)`均成功各收3 A，对应元数据Read返回`NotFound,NotFound,v0:b1,v1:b0,v1:b0`，最后0B回收、B20、快照一致；修复了上一研究第三轮硬编码CAS None造成的失败。STALE_CAS专门在第三轮VM真Read到v1后研究者用正常付费CAS写为v2，VM原本预期v1的普通Write真实`Conflict`且不覆写新v2、不继续GC、不谎报记忆；RELEASE_AND_REPLAY解除pin后第三轮真GC3×Ok释放**834B**，两份中途snapshot的续行最终完整state+audit逐字节相等，原B20不重算；METADATA_GUARDS真付费1B自有对象meta Read3 A且ACL/path/bad version拒绝，版本超界/伪owner通过**受信边界夹具直接修改**而非生命自创。
- **边界**：新元数据read能使同一VM正确使用最新CAS版本，不消除read→write竞态（已真拒绝）；内核未提供全世界引用安全、个体没学到动态等待策略，研究者仍给固定三轮、三个候选、容量阈值和实际pin轨迹。G0 GO/G1/G2 NO-GO、E001未校准/预注册。下一轮优先跨epoch真实写入频率校准失败成本，再研究候选和目标内生发现。


### 2026-11-23 — S29：无害写入时序造成可测证书失效成本；跨 epoch VM 记忆重放原生4/4

- **先冻后跑**：[实验协议和完整四臂账本](./experiments/E001/DEV-S29-CROSS-EPOCH-CERTIFICATE-CHURN-PROTOCOL.md) @395c2ba，重用上一S29同一根hash `88af07516c0e101b260b7950f0e1de5afbfeb5e556952b4bd26b26a2be306815`、真实P2 world129/protocol42/B20、A12000/epoch、实际三候选834B与三轮上限。本轮**不新增世界动作、VM指令或内核功能**；只改变一次研究者合法付费的无害1B自身Write相对有偿Index的发生时序。测试通过原生真实index/GC/CAS/audit，绝不直接改认证索引制造冲突。
- **原生首次全绿**：[Actions #45151720848](https://github.com/iFrankStudio/OCEE/actions/runs/45151720848) @e12f53c9 **completed/success，独立Rust4/4、无红例**。QUIET Index1次1667A、GC3×Ok8A、VM A191、动作A1693，释放834B；BEFORE_INDEX研究者1B正常Write3A在Index前，仍Index1次1667A、GC3×Ok8A、VM A191/动作A1696，释放834B；**AFTER_INDEX同样1B正常Write3A在Index后，使证书真正失效→GC Conflict0A、VM真记risk1 v0→冷却写risk0 v1→第三轮Index再付1667A后GC三次Ok，VM A509/动作A3379，也只释放834B**。同一写入内容和同样834B释放，时序一变**维护VM+世界动作A从1887增到3888，多耗2001A（其中额外扫描1667A）**，构成真实保守失效机会成本反例。
- **EPOCH_GAP**将AFTER_INDEX真risk1 v0之后的同一活动VM挂起、只由外部宿主合法`tick(None)`把世界从epoch2推到epoch3，保证期间VM continuation不动；从跨界snapshot复制世界双支，第三轮真正重Index/GC回收834B，VM A509/动作A3379、B20、状态与全部audit逐字节相等、无研究者重新激活/越权执行。它只说明**已经存在的持久VM和普通风险记忆可跨资源epoch继续**，不是个体自己触发唤醒。
- **研究界限与下一决策**：所有候选、固定压力门槛/退避和激活均由研究者给出，仍然没有概率学习、候选自动枚举、一般活引用GC或开放式演化。内核认证索引目前对正常无害Write也全球保守失效；接下来最有信息价值的是比较**同样严格安全、付费窄粒度更新**与原全失效路径，并保留能把活hash隐藏在长度≥64B载荷/异步引用里的失败对照；不可因更低开销而放宽安全。**G0 GO、G1/G2 NO-GO，正式E001尚未校准/预注册**。本轮单专项4/4不代表全workspace已复验。


### 2026-11-23 — S29 严格短写入认证索引保留：原生4/4，固定世界维护A开销从3888降至1888

- 前一轮[认证索引时序实测](./experiments/E001/DEV-S29-CROSS-EPOCH-CERTIFICATE-CHURN-PROTOCOL.md)确定正常无害的1B写入若发生在付费Index之后，原有内核一律撤销证书，下一次GC Conflict并重索引；同样回收834B却付出VM+Action共**3888 A**，相比索引前写1887A浪费2001A。先冻结[严格短写入安全与付费协议](./experiments/E001/DEV-S29-CERTIFIED-SHORT-WRITE-PRESERVATION.md) @443dac77，再在既有`Action::Write`实现**必须当前认证actor/active一致、键及new bytes各<64B、只限普通/self非active/versions/tools路径**的小写入保留条件。由于原认证索引分别对key和bytes以64B原始ASCII hash窗口扫描，严格短字段不可能新增被认证的引用，保留旧`referenced`超集安全；实际成功的短写入在原有Write费用外**额外收费1 A**。其余索引失效、GC活引用保护和VM根完全不变。
- **[原生Actions #45156166086](https://github.com/iFrankStudio/OCEE/actions/runs/45156166086) @da177c8e completed/success，独立Rust4/4，首次全部成功、无本轮红例。** 四臂使用上一S29相同真实P2 world129/protocol42、原子Len修复B20、研究者安装三旧版834B与同一维护根hash`88af07516c0e101b260b7950f0e1de5afbfeb5e556952b4bd26b26a2be306815`。SHORT_WRITE首次真实Index1667A后1B正常写4A，证书仍有效、VM索引**1次**、GC三次Ok删除834B，VM A191/Action A1697，累计**1888A**；较旧AFTER_INDEX **净省2000A/51.4%**，相对于上轮BEFORE_INDEX仅多1A显性审查费用。FAILED_CAS合法短写4A后过期None CAS Conflict/0A、不改证书与数据，也只一次Index，VM191/Action1697。
- **安全反例真实通过**：LIVE_HASH_PIN长度64B原始hash真Write6A仍废证书，GC两次Conflict、风险最终1v2、三个旧版本全部保护不删，VM558/Action3367。OLD_REF_REMOVED先真Write64B活引用、真实Index记录被引用，再CAS改成1B短值4A仍保留原引用**超集**，快速GC依旧Conflict（不误删）；只有研究者重新付费建第二份完整Index证明未引用后，才真GC8A释放279B。其直接GC是研究者施加的合法世界动作，不能宣称VM自动生成重扫描策略。全部4臂P2 B20不重复、原root不重启、终态snapshot/hash一致。
- 当前是受限的**索引保留**，非可从对象中删除旧引用的精确增量GC，也未提供独立epoch自唤醒、候选自动发现、长程写频收益、通用编码引用安全或开放演化；只验收专项Rust4/4，不能替代全workspace。后续若研究逐对象差分/多持有者安全，必须保留多对象、长hash与消息引用负例并严审A+索引内存成本。**G0 GO、G1/G2 NO-GO；正式E001未校准、未预注册。**


### 2026-11-23 — S29 付费逐来源引用差分：原生首轮6/6；同一279B回收节省1661 A

- 从最新`65bdff70f0c7`（上轮安全短写入认证4/4）出发，**先冻结**[可证伪实验与安全/计费协议](./experiments/E001/DEV-S29-PAID-SOURCE-DELTA-CERTIFICATE-PROTOCOL.md) @91b00ed6，保留原`Write`及其收费、`IndexVersionRefs`及其原有byte-scan收费/64字节ASCII完整hash安全边界。新增**显式**`Action/WorldCall::WriteIndexed`原子有偿普通自身CAS，内核可信索引在同一付费全扫描中保留**逐对象来源的hash集合及独立消息/邮箱来源**；新CAS时付费扫描新key+bytes、重算所有旧/新来源安全并集，实际在同tick更新对象、storage和认证证书，拒绝旧快照无来源证书、无效actor/active、CAS冲突、Gas/Quota，且不可写active/versions/tools。没有把未收费的`/self/`任意数据当内核证书。
- **[原生Actions #45163504990](https://github.com/iFrankStudio/OCEE/actions/runs/45163504990) @7e15bf9e，6个Rust独立job 6/6完成成功，第一次尝试无编译红例/机制红例。** 固定真实S29/P2 world129/protocol42已修复任务正确B20，3旧程序已安装，长臂相同root `88af07516c0e101b260b7950f0e1de5afbfeb5e556952b4bd26b26a2be306815`。**LEGACY_WRITE**先真实放入64B唯一hash并付费Index1668A后，普通旧Write4A撤掉引用，旧证书仍保守Conflict；只有第二次完整索引后才能真GC279B，总世界动作 A **3353**，索引2次。**DELTA_SINGLE**除选用新WriteIndexed10A外条件相同：一次Index1668A后立即合法移除唯一引用、无需第二索引且真GC279B，总世界动作 A **1692**、索引1次；研究者动作区间**节省1661A/49.5%**，并没有对原VM A（四个研究者长臂均0）声称获益。
- **DELTA_MULTI**两处不同对象都真持有同一64B hash，第一次IndexedWrite11A移除其中一处后真实GC Conflict，第二处IndexedWrite10A成功后才删除279B，全程仅一次Index1670A；**DELTA_ADD_AND_STALE**以15A真实长写新增活hash→GC Conflict，旧None CAS Conflict/0A不改对象/证书/storage，再10A合法CAS删除引用→GC279B，一次Index1667A。**TRANSPORT_LEASE**独立小世界真实发送到C的在途消息hash被完整Index收录，本地移除引用的IndexedWrite5A不能取消传输来源，真实GC Conflict，无误删。**VM_DELTA**单独真正用`WorldCall::WriteIndexed`启动合法类型化VM root `7793c768...40a`，单次真实活动VM执行获原子Ok回执（动作10A+VM7A），之后真实GC回收279B且原P2 B20不重算。所有长臂快照hash一致。
- 内核证书真实序列化大小示例初次为 **1606–1881B**，未作为个体存储配额入账；它仍是有限的**已安装程序原始64B哈希引用模型**，无法证明外部异步服务、任意编码/间接引用或一般追踪GC安全。本轮新能力由研究者设计/提供，不是个体自发创造；内生候选发现、动态目标选择与开放式演化仍无证据。E001并未校准/预注册，G0 GO、G1/G2 NO-GO；专项6/6不代表整个Rust workspace全量复验。完成后专项workflow回归手动重放。


### 2026-11-23 — S29 真正目录驱动的受限候选发现：原生4/4，开始摆脱写死程序hash

- 从实时main `7557be27e3ca`起，先冻结[有偿自身版本目录和有界VM发现协议](./experiments/E001/DEV-S29-PAID-OWN-INVENTORY-BOUNDED-DISCOVERY-PROTOCOL.md) @8698549b。此前逐来源认证GC证明内核安全但固定旧版本hash仍由研究者写进活动维护程序。此次保持原`Action::Read`/VM原语集合、版本索引及GC不变，在世界内核增加严格`Read("/sys/self-version-catalog/<page>")`：只列actor真正拥有的完整64位hex路径版本及**物理字节长度**，排序按`(size,hash)`，8条/页、最多1024条、canonical页号0..127，真实有偿 `2+ceil((全部拥有版本key长度+本次响应字节数)/64)` A，不暴露内容字节、活引用状况或回收建议。错误页BadRequest、配额不够Quota、A不足Gas，无oracle。
- **[原生Actions #45180137615](https://github.com/iFrankStudio/OCEE/actions/runs/45180137615) @b905f6bdd02，4个独立Ubuntu Rust native jobs 首轮4/4 completed/success，每个1 passed/0 failed。** 活动VM原始字节码固定`49d4f015181b1773b0ae77e38d53ba90c89ef85e985d8c163e7dedd7fb1c2036`，**从不包含任意三旧候选的64B hash字面常量**：真`Read`第一页并解析动态JSONArray、真`IndexVersionRefs`受费建索引、类型化静态验证的`RepeatBounded(limit=8)`尝试`ReclaimIndexedVersion`，遇实际Conflict/NotFound尝试下一个，其它失败中止、第一笔真实GC成功才停。
- **CATALOG_CONTRACT**真实世界目录10个所有者自有版本、页0收费27A，空页/不规范页/资源耗尽边界均守约；gas注入属于研究者负例。**DYNAMIC_RECOVERY**由活动VM从真目录发现目标`f0b263...69837`而非硬编码，按真实内核GC序列`Conflict,Conflict,Ok(8A)`回收**276B**，VM A700、Action A1702（本次维护片段共**2402A**），B20不重复。**PIN_AND_CONTINUATION**研究者先真付费写64B活pin，VM自动发现它但真GC Conflict，不误删，随后发现并回收另一个279B；克隆真实index后snapshot继续两份完整audit及终态逐字节一致，VM A788/Action A1709。**INVALIDATED_CERT**活动VM真Index之后研究者付费写64B新引用废掉证书，后续目录8项GC全部Conflict/0A、不删任何版本，B20、真实快照一致。
- 这次真正推进的是“**有偿感知已有资产→运行时产生候选身份→受权工具尝试并按错误反馈换目标**”。此前目标identity固定在bytecode中的实验做不到这一点。但**总维护费用仍昂贵且单次只收回276B**；不是能量/存储综合效率证明，不能将700A、1702A与276B直接相减。第一版VM固定只取目录第0页、最多8项、按字节长度升序选首个获准回收，没评估哪一条值得付索引费；候选旧工具仍由研究者安装，**维护root启动也仍是研究者触发**。这些是能力扩展而不是自发形成维护目标或开放式演化。只做专项4/4，没有声称全Rust workspace回归。**G0 GO、G1/G2 NO-GO；E001尚未校准/预注册。**


### 2026-11-23 — S29 单体从“知道怎么GC”迈向“决定何时启动”：资源门控原生5/5与0收益反例

- 实时main `68e8fe661d5e`上先冻结[受费资源/算力门控协议](./experiments/E001/DEV-S29-RESOURCE-BUDGET-TRIGGER-PROTOCOL.md) @f25a225d。继续复用既有付费`Read("/sys/resource-status")`、`Read("/sys/self-version-catalog/0")`、`IndexVersionRefs`、`ReclaimIndexedVersion`，**未新增特权动作、GC白名单/新VM指令**。研究者给出固定、类型验证的单次活动VM根hash `534ab1585013d573b310fa3aa5b8aa462d6e18c486c8be55047167bfee4bc2c8`，不含任何研究者额外3旧程序的原始64B hash字面值。VM先真付2A读取自己的used/capacity/a_available，借原通用`Sub/Div/EqU32`在物理空闲<4096B且剩余A>=3000时才付费扫描目录、Index并按真实Conflict/Ok有界尝试GC；门槛值是**研究者固定启发式**，不是个体学习。
- **必须保留的首轮原生红例**：[Actions #45202464053](https://github.com/iFrankStudio/OCEE/actions/runs/45202464053) @2af48822**4/5**，其中PROTECTED_ONLY仅给3条人工追加版本施加有偿pin，却仍存在真实目录其它合法可回收版本，VM按内核真Ok返回1；是研究夹具误假设所有版本均受保护，非GC安全失败。先在实验协议冻结修订，再仅修改该负例夹具：对当前**所有10个真实自有版本**分别真付费写64B活pin（640B物理新增），不触碰4个已通过arm、原VM bytecode、4KiB/3000A阈值或不回收的判据。修正后 **[Actions #45203295504](https://github.com/iFrankStudio/OCEE/actions/runs/45203295504) @8560eb39，五个独立native Rust jobs首次全矩阵5/5 completed/success、无第二轮红例。**
- **PRESSURE_EXECUTE**（实际可用2KiB）VM真读资源A余额11157、再真目录/Index各1，GC`Conflict,Conflict,Ok(8A)`，真回收**276B**，维护VM**780A**+世界Action**1704A**=**2484A**。同根/同已安装程序/同P2 B20的 **SPARE_SKIP** 仅增加配置空闲到8KiB，VM真读资源2A之后**0目录/0Index/0GC**、返回U32(0)，VM82A+Action2A=**84A**；这一可控相似世界中避开了**2400A**的当前维护开销，同时也没有释放可回收的276B。**LOW_A_SKIP** 不是硬改A余额：研究者真调度下一epoch`ResourceChange(A,2400)`，跨真实epoch发放后资源传感器观测剩余2394A，空间2KiB，但策略仍0目录/0Index/0GC，VM81+Action2=**83A**。
- **RACE_AFTER_SENSOR**：VM真读资源后、下一步目录前研究者真付费写64B hash pin，VM索引现实活引用，GC序列`Conflict,Conflict,Conflict,Ok(8A)`，被pin对象拒删、另一279B版本真回收，VM868A+世界Action1711A（含研究者pin6A）。**PROTECTED_ONLY** 全部10个实际版本有活pin，sensor真见空闲1408B而A余额11097，VM仍花**1236A+1709A=2945A**于1目录/1Index/8次全Conflict/0A，**回收0B**。此负例是本轮最关键科学约束：**空间短缺与算力充足并不意味着存在可回收收益**；前置研究者写10个pin的费用不在上述维护区间内。
- 全五组真实前置S29/P2已完成原正确工作且B20不再重复，根只由研究者初次受费激活一次，终态经原生世界snapshot恢复重跑full audit一致；专项Rust5/5不冒充全workspace新一轮通过。该策略也没有自主估价资源、多epoch自动唤醒、自学阈值或开放式演化，更没有把A与B当可相减的统一价值单位。最有价值后续研究应瞄准**在真实所有版本受引用时，如何有偿判断无收益并避免2945A白费**，不得添加隐藏的可回收性oracle。**G0 GO、G1/G2 NO-GO；正式E001未预注册/校准。**


### 2026-11-23 — S29 真实零收益记忆、单轮冷却、重新尝试与观测不可区分对照：原生4/4

- 从实时`432fd9fba011`读取后，先冻结[零收益持久记忆/错失机会/付费观测对照](./experiments/E001/DEV-S29-PAID-DRY-RUN-MEMORY-COOLDOWN.md) @a6b62cd2，之后仅修改定向S29 Rust实验测试及新增单次受控专项workflow；**没有内核/VM原语/权限改动**。研究者安装并只激活一次有界Typed VM程序 `121b42f678b80df15b7ca17c7c6adb6a99ca5d3c490cc03288fedaee1fe21506`，程序里**没有任意额外旧版本的64B hash常量**。在上一轮已有资源/算力门槛+有偿目录动态候选上，加入`RepeatBounded(3)`轮次及现有`RepeatScoped(3,aggregate24)`每候选有限搜索；真实世界Read/Write普通`/self/maintenance/s29-dry-run`记录上轮8次Conflict/NotFound造成的零回收，下一轮付费CAS b1→b0跳过昂贵目录/索引/GC，第三轮真实付费重试，读取和写入的CAS版本都由内核真实提供，不能写死。
- **[原生 Actions #45206316515](https://github.com/iFrankStudio/OCEE/actions/runs/45206316515) @`32de261fa5517acaa0e601f659b6ef212caeffd8`，首次4个独立Ubuntu Rust job全部1/1 success，4/4、无本轮红例。** `DRY_REPEAT`在真正P2完成/B20、研究者有偿将所有10版本64B完整hash分别pin住的世界内，单个活动VM的第一轮真目录+真索引并8次GC Conflict、付费创建b`1` v0；第二轮真实付费读记忆并合法CAS b`0` v1，**零目录/零Index/零GC**，该轮仅**VM77A+Action8A=85A**；第三轮再真目录+真Index+8次Conflict后合法CAS b`1` v2。3轮共**2**次完整索引、2次目录、**16**次GC Conflict/0A、0B释放；首轮VM1311+Action1713=3024A，三轮总VM2773+Action3442=6215A。85A相对3024A是不同记忆状态下的轮次比较，**不是同一起点两组随机化净效用实证**。
- `RELEASE_AFTER_COOLDOWN`与同一个根、同一个true all-pinned初态一致，第二轮付费b0 CAS后由**研究者**以真实付费64B→64B CAS解除其中一个pin；第三轮真正重建Index、动态目录发现并回收该旧程序**276B**。两次Index、两次目录、GC总11次，VM2246A+世界Action3448A（含研究者释放写），快照完整audit一致。证明冷却没有永久停摆，也说明第二轮存在延迟响应的**机会成本**，不能将冷却吹成实时神谕。
- `STALE_MEMORY_CAS`在第一轮b1v0真提交、第二轮VM真读到v0后，研究者抢先真CAS改为b0v1，VM手中旧v0作出的后续b0写得到真实**Conflict/0A**并终止，不覆盖v1、不重新索引或GC，状态恢复逐字节一致。体现普通版本化对象记忆仍需安全CAS，而非Agent特权内存。
- `PAIRED_INDISTINGUISHABILITY`特别重要：从完全相同真实S29/P2 snapshot分叉、为每个已安装10版本付费创建10条**完全等长64B**普通self对象。一个世界值为真hash、另一个值为无哈希填充。两者实际storage/容量/A/对象路径及数量、已安装版本数完全相同，付费`Read("/sys/resource-status")`、`Read("/sys/self-version-catalog/0")`响应**字节完全相同、收费相同**。之后两个世界各自真实付费Index均收费**1680 A**，同一已安装276B旧程序在有活引用的世界真实GC Conflict，另一无活引用世界真实GC Ok并释放276B。证明只看这两个**低信息量接口**无法识别所有可回收性差异，不能假装阈值能消除第一次完整引用扫描；**不证明所有可能的其它付费观测都不可区分**。
- 以上为**受限个体执行研究者写好的有偿反馈策略**、3轮同一单次活动root的工程证据；没有证明自发生成资源维护目标、自动多epoch唤醒、自己学习阈值/反作弊条件、A/B可比较收益、通用不透明或外部引用回收，更没有开放式演化。只做4臂原生专项，不冒称全workspace重跑。**G0 GO / G1、G2 NO-GO；正式E001尚未校准/预注册。** 本轮CI专项证据归档后恢复手动运行。

### 2026-11-23 — S29 三种维护策略真正配对的任务效用初步证据：原生2/2，冷却收益与失败并存

- 根据并行研究者提交的[预冻结真实生命周期效用协议](./experiments/E001/MAINTENANCE-MATCHED-LIFETIME-UTILITY-PROTOCOL.md) @2479c862，第一次实现**同一真实世界fork后的三种策略配对**：ALWAYS_RETRY、NEGATIVE_COOLDOWN（既有一轮持久反馈冷却）、NO_MAINTENANCE。实验保持真实S29 P2 seed129/protocol42任务已正确完成B20、三策略相同12个已安装自有版本、所有真实64B hash付费pin，初态共同快照SHA `2335c22c116ae488856f8d5b6a5aa0897ccad65f91bc0b5c4eb78a6cb098b522`、物理存储使用128278B/容量128473B、A=10010，三个策略根由研究者以**等价的真实受费CAS**各自激活。外部相同pin CAS定时步2165、相同280B合成有用存储任务步2230、终止步2320；从每支快照恢复的全文audit和状态逐字节一致，研究者动作与个体VM/维护成本在审计账本分开统计。
- CI红例明确保留：[首次Run #45222742304](https://github.com/iFrankStudio/OCEE/actions/runs/45222742304) 两臂因测试报告缺失format参数**编译失败**；修复后[Run #45222915650](https://github.com/iFrankStudio/OCEE/actions/runs/45222915650) ALL_PINNED通过，逆境臂错误要求冷却一定省A而在**已经执行真实世界后**触发测试断言失败。这违背原冻结的H2逆境假设，故只把省A的断言收缩到ALL_PINNED，不改原策略/放行条件/任务时刻。改正后 **[Actions #45223821160](https://github.com/iFrankStudio/OCEE/actions/runs/45223821160) @865fc750a9，2个独立原生Rust job全部success、每臂1 passed/0 failed，2/2**。实验详细失败日志、提交sha、event tape SHA与安全边界已入对应协议。
- **ALL_PINNED真正配对持续无收益**：ALWAYS 3次真Index、22次GC Conflict、VM3873A+维护Action5738A=**9611A**，回收0B、外部280B任务失败；COOLDOWN 2次真Index、16次GC Conflict、VM2771A+维护Action3828A=**6599A**，回收0B、同样任务失败；NONE没有GC/Index，VM2A、外部任务也失败。在这个**同snapshot、同事件时间**的控制下，冷却避免了**3012A/31.34%维护算力**，但并没有提高实际完成任务数；无维护总能省更多A也不能改变存储拒绝。
- **RELEASE_BEFORE_SECOND真实逆境**：同一真实pin于固定步2165被研究者付费改成无哈希等长填充。ALWAYS第二轮真实索引+GC释放276B，**第2230步280B任务成功**（真付费20A），维护VM2251+Action3823=**6074A**；COOLDOWN因负结果记忆跳过第二轮，此时存储任务被**Quota/0A拒绝**，第三轮才真实GC释放276B，但错过任务已不可挽回，维护VM2332+Action3828=**6160A**，即**多花86A却少完成一个任务**；NONE始终不释放、任务失败。所有世界仍原P2 B20，不重复收费、不越权删除。
- **特别重要：这不是自适应冷却已经学会策略优化。** 两臂只有seed129、单次externally activated有限策略，任务是研究者制定的合成存储artifact；本轮仅实现[完整预冻结协议](./experiments/E001/MAINTENANCE-MATCHED-LIFETIME-UTILITY-PROTOCOL.md)要求的五张事件tape中的两张，**其余3张、至少3 seeds、证书索引的未计费峰值字节审计尚未完成**。现有endpoint证书序列化最终均0B不能冒充peak；策略root大小11,561/11,635/248B虽均事先共同付费安装，尚未纳入长期全生命周期负担。长程综合资源收益和自发维护目标完全未证实。**G0 GO / G1,G2 NO-GO，正式E001未校准/预注册。** 定向2/2不能当全Rust workspace回归。


### 2026-11-24 — 种子依赖问题定位、真实P2规则泛化3/3及独立修订协议下三种子×三时序9/9

- 按上轮[生命周期效用原冻结协议](./experiments/E001/MAINTENANCE-MATCHED-LIFETIME-UTILITY-PROTOCOL.md)继续扩展原单种子2/2实测，新增真实World每tick后受信任引用索引序列化大小高水位（**不是收费的个体存储，也不是OS峰值**），冻结种子129、130、131不改。**首轮跨种子原协议红证据保留**：[CI #45308566833](https://github.com/iFrankStudio/OCEE/actions/runs/45308566833) @62f95ee9仅种子129的2个tape通过，**2/6**；seed130/131四job均在维护实验**开始前**被种子129独有的Len程序hash固定断言拦下，GC/冷却未运行。[第三个“冷却后再释放引用”CI #45308748855](https://github.com/iFrankStudio/OCEE/actions/runs/45308748855) @749943c7同样仅seed129通过**1/3**、其余二seed死于相同前置hash断言。两个失败工作流保留，不以修订版本替代原失败。
- **独立受控诊断不是研究者更换P2程序**：[原生P2基线诊断 #45309166319](https://github.com/iFrankStudio/OCEE/actions/runs/45309166319) @2df4827b **3/3全部成功**。同一原真实S29/P2 VM根根据公开两样例自行发布seed129`Len(0)`、seed130`XorFold(0)`、seed131`ByteAt(0,1)`（指令寄存器1实际为0，即第一字节）。三个VM都真实发布一份可内容寻址canonical Program、REGISTRY版本1、原真实隐藏样本预测分别**5/44/34**与独立隐藏oracle吻合、P2服务B**20**、完整snapshot一致。原来的Len断言不是多种子P2成功标准：seed130环境正需要seed129曾视为“可丢弃”的XOR程序，固化相同GC候选在跨种子上会错误地选择活工具。
- 因而**先单独冻结**[种子感知的真实未引用候选v2协议](./experiments/E001/DEV-S29-MATCHED-SEEDED-ACTUAL-ORPHAN-UTILITY.md) @32084f41，原基线test/原预注册和原两失败runs均不“改成成功”。v2在P2真正完成后仍只使用固定原三个已安装候选，跳过真实P2已安装的重复immutable程序，从三候选中按预定规则排除seed自己的真实注册工具，再选**物理字节长度最小、hash字典序**的候选；研究者在**丢弃的fork**用真付费目录/解除pin/Index/GC验证仅该目标确实安全可回收，绝不把目标hash放进VM根。三种seed原始恢复根不变，三条策略根均事先安装并各分支真付费切换，研究者对相同时间的64B pin CAS与280B合成有用任务，在每个seed三个policy分支完全匹配；每个世界都真实P2 B20、账本与snapshot重放一致。
- **[独立v2原生Actions #45309723967](https://github.com/iFrankStudio/OCEE/actions/runs/45309723967) @92ad1693d87c，9个独立Ubuntu Rust job首次全部completed/success，**9/9**，每个job在一枚seed×tape中运行ALWAYS/COOLDOWN/NONE三个真正world分支。seed129选XOR旧版`f0b263...` 276B，seed130选`6820d121...` 279B，seed131选XOR`f0b263...` 276B；它们不是所有种子都硬写同一个hash。完整实验具体费用与event-tape hash见独立v2协议。
- **持续无回收机会ALL_PINNED（按同快照同外部时刻）**：seed129 ALWAYS维护**9611A** / COOLDOWN **6599A**（少**3012A**）；seed130/131 ALWAYS**9585A** / COOLDOWN**6582A**（各少**3003A**）。两者任务**均未完成**、没有任何GC成功；NONE维护2A、任务也失败。COOLDOWN对比ALWAYS少1个完整索引（3→2），这并不是改善任务完成数。
- **释放发生在第二轮冷却前的逆境RELEASE_BEFORE_SECOND**：seed129 ALWAYS维护6074A、完成真实280B任务，COOLDOWN维护**6160A**、任务Quota拒绝（+86A且**失去一项按时任务**）；seed130 **6057A/6143A**、seed131 **5969A/6055A**，两组同样+86A并同样失去按时任务。两种维护策略最终都经真实GC回收各自276/279B，但冷却是在截止以后才完成，不能后补计成功；NONE任务也失败且不回收。
- **释放发生在第二轮冷却之后的RELEASE_AFTER_COOLDOWN**：seed129 ALWAYS维护**9350A** / COOLDOWN**6161A**（少3189A）；seed130 **9324A/6144A**、seed131 **9236A/6056A**（均少3180A）。同一真正已付费的第二轮记忆b1→b0 CAS之后，研究者才在固定第140步解除指定pin，三个seed下**两种维护策略都在task第205步前真回收并成功提交280B任务**，NONE未维护且任务失败。该证据不能解读为个体“预测”了发布引用的时刻。
- **原无证书内存峰值的缺口部分关闭**：seed129完整Index实际**post-tick可序列化高水位3382B**，seed130/131**3155B**，而终态证书常因后续真实写入失效、序列化0B；NONE从头到尾0B。全部属于内核可信证书、**当前未向个体存储Quota计费**，不得把存储回收276/279B讲成全系统净存储改善。策略程序自身字节数11,561/11,635/248B已经作为实际世界对象共同付费安装，但尚未摊销为长期收益。
- 上述成功为**独立定义的v2测试协议下“三种子×三张研究者设计的时序”**，不是原固定工具哈希方案跨seed9/9，也不是预冻结5tape×3seed的完全实现；尤其多来源/在途消息保护与无任何可回收版本的另一独立tape未在本轮进行。没有开放式演化、内生目标或自主调整冷却时长的证据，不能从3个seed和一个合成280B任务外推普适性能。**G0 GO，G1/G2 NO-GO；E001正式校准/预注册尚未完成**。仅定向原生Rust，不声称本轮全workspace cargo test成功。

### 2026-11-24 — S29 配对补充两类环境，原生6/6
[补充协议与实测](./experiments/E001/DEV-S29-MATCHED-TRANSPORT-AND-PRUNED-INVENTORY.md)，[CI #45312329768](https://github.com/iFrankStudio/OCEE/actions/runs/45312329768) @c49ce017，6/6 independent native Rust success，日志逐项检查，无红例。三seed真实P2 B20、三策略配对与snapshot/audit复演。
- MIXED：同一个体两普通64B对象引用，研究者在共同初态设置delay95，固定解除本地pin并真SEND64B。ALWAYS第二轮真证书含transport_refs且目标GC Conflict，第一项280B任务三策略均Quota；网络引用真正消失后两维护策略第三轮回收276/279/276B、第二项任务均Ok，NONE两项失败。维护A（ALWAYS/COOL）129=9356/6164，130=9330/6146，131=9242/6058，省3192/3184/3184A；外部15A/C64，任务成功再20A，无额外B。可信index序列化峰3487/3260/3260B未计个体Quota。
- PRUNED：真实前置清理834/558/555B、准备A1337/1315/1315，剩余各9个版本在独立丢弃fork逐个真GC验证Conflict。容量按实际used+195重设，自然epoch grant；所有任务Quota，无新增GC。维护A 9546/6556、9543/6553、9543/6553，均省2990A，index峰2703/2703/2702B未收费。不能据此声称原容量不变时前置清理无效。
- 与独立v2的9/9合计15组（五类时序×三seed），并非失败v1被回写成通过。合成任务和外部激活、固定冷却规则仍是研究者提供；无自主策略学习、跨多epoch生命效用或开放式演化证据。**G0 GO / G1,G2 NO-GO**，正式E001尚未校准/预注册。下一研究应转向付费经验对后续维护选择的实际因果影响，而非继续堆叠固定GC测试。

### 2026-11-24 — 五类配对证据汇总与下一项自有任务反馈试点
- 汇总独立修订v2的9组和补充6组原生成功案例（均为不同seed×人为时序，非统计抽样）：ALWAYS累计维护131351A，COOLDOWN累计94512A，少36839A（28.05%），NONE累计30A。每策略实际18个外部合成存储任务尝试，按时成功分别9/6/0；COOLDOWN额外丢失3项均来自提前解绑逆境。汇总不提供真实生态概率，不含外部费用、研究者安装及已清理目录前置成本，不能将A和存储字节合并为单一效用。
- 下一项[自有任务Quota反馈试点](./experiments/E001/DEV-S29-TASK-QUOTA-OVERRIDE.md)先冻结 @0707baf4、后实现 @f2f680b4、触发[定向原生CI #45318054188](https://github.com/iFrankStudio/OCEE/actions/runs/45318054188) @716a5109。两策略同一真实付费b1负反馈初态，VM自身尝试同一280B合成写任务，仅真实Quota触发读取记忆；固定冷却与Quota覆盖分支对照，分别RELEASED/PINNED/SPACE三臂（seed129）。成功GC后仍须真实任务重试Ok才能返回成功。**CI尚未验收，不能当作通过。** 下一次从此run真实日志继续；固定实验目标、外部激活和研究者编写的策略仍不构成自主学习。G0 GO / G1,G2 NO-GO；E001正式校准/预注册未完成。

### 2026-11-24 — 自身任务Quota反馈闭环，原生3/3
[协议与实测](./experiments/E001/DEV-S29-TASK-QUOTA-OVERRIDE.md)，[CI #45318054188](https://github.com/iFrankStudio/OCEE/actions/runs/45318054188) @716a5109：3独立native Rust作业均1 passed/0 failed，实际日志逐项核查。seed129真实P2 B20，先真实付费失败扫描形成b1/v0，后两预装策略从共同快照等价激活。VM自身真实Write 280B，不是研究者代写。
- RELEASED：固定冷却首写step4099 Quota/0A，非任务费用138A，b0/v1，未完成。Quota覆盖首写Quota后读取真实记忆、资源门控、真目录/Index、3 Conflict后真实GC276B，step4108同一VM再写Ok/20A并完成任务；VM1016+非任务动作2332=3348A，b1/v0残留。比固定多3210A非任务费用换来一项完成任务；index序列化峰3688B未计Quota。
- PINNED：固定138A/无Index/任务失败；覆盖3747A/1Index/8Conflict/0B/任务失败，额外3609A无效；b1/v1，index峰3848B。SPACE：两策略第一次即Ok/20A，VM41A，无维护和记忆变更，b1/v0。所有版本安全、注册工具正确、全audit/snapshot/峰值复演一致、B20未重复。
- 该覆盖规则在合法b1和b0下均进入资源门控，本质是Quota门控重试，尚非根据历史学习的策略；成功后保留旧失败记忆是明确缺口。下一步研究真实自身任务历史的付费更新，归一化未来物理状态后比较同一政策是否因历史不同做不同选择，并设忽略历史的对照。不把预设目标与规则描述为自发学习。G0 GO / G1,G2 NO-GO，正式E001未校准/预注册；仅定向原生回归。

### 2026-11-24 — 自有任务真实经验跨后续决策的因果试点（待验收）
- 先冻结[独立经验因果协议](./experiments/E001/DEV-S29-EARNED-TASK-HISTORY-CAUSAL-TRANSFER.md) @347c76ce，再实现 @83c34d79、专项CI [#45318793560](https://github.com/iFrankStudio/OCEE/actions/runs/45318793560) @ac986f79。seed129真实P2、同一付费安装三新根；同训练根真实280B任务先Quota，成功支真GC+重试Ok后由VM真付费创建b0/v0；失败支真耗尽搜索创建b1/v0。没有研究者写记忆。
- 两训练历史均通过付费合法操作归一化旧任务对象（空/v1）、被清理版本实际重新安装、同pin CAS v2、同容量used+195、自然epoch grant；断言当前所有对象内容和版本除记忆同长度同版本的不同字节外相等，节点资源、mailbox、pending、step、库存相等。随后同一probe根对两真实历史完成新280B任务，另设忽略历史的quota覆盖根；RELEASED/PINNED两环境，各四probe分支、90tick horizon，完整重放与费用账本。若环境改变时旧记忆导致错误跳过，保留反例。仍为研究者编写二元反馈策略，并非策略自发学习。
- **本试点只已提交与触发，尚未验收。** 继续时先查该run真实日志和最新HEAD，不预判通过。G0 GO/G1,G2 NO-GO；正式E001尚未校准/预注册。

### 2026-11-24 — 真实任务恢复后的负反馈修订（待验收）
- 在历史因果试点#45318793560尚运行时，独立冻结[修订协议](./experiments/E001/DEV-S29-RECOVERY-MEMORY-REVISION.md) @5cd42ce8，后实现 @2268f8fa，触发[定向原生CI #45319202821](https://github.com/iFrankStudio/OCEE/actions/runs/45319202821) @9681065a。真实seed129/P2 B20、共同付费b1/v0失败经验；同一280B任务、实际Quota后维护并重试。原STALE覆盖根与新REVISE根对照，仅REVISE在真实重试Ok后付费读既有记忆meta、校验b0/b1、按真实版本CAS b0。任务已成功则CAS失败不能倒算任务失败；二者分别记账。
- RELEASED/PINNED/STALE_CAS三臂，第三臂按真实成功后的meta读取回执插入研究者b1/v0→b1/v1付费CAS，要求VM旧版b0 CAS实际Conflict、不覆盖新记忆、不重复写且已成功任务仍保留；单独记录外部A、任务A、VM和非任务动作A、索引序列化峰、完整审计与snapshot复演。没有修改旧已通过试验或正在运行的因果试点。新试验尚未验收，不宣称通过。G0 GO/G1,G2 NO-GO，正式E001尚未校准/预注册。

### 2026-11-24 — 真实历史因果2/2与恢复后记忆修订3/3验收
- [历史因果原生2/2](https://github.com/iFrankStudio/OCEE/actions/runs/45318793560) @ac986f79；[完整协议与实测](./experiments/E001/DEV-S29-EARNED-TASK-HISTORY-CAUSAL-TRANSFER.md)。真实seed129/P2 B20；同训练根实际任务Quota后，成功历史GC276B并step3085重试Ok/20A、step3086真写b0/v0；失败历史8次Conflict/0B、step3090真写b1/v0。合法付费归一化28/12A，实际对象map（除1字节记忆）、节点资源、step、库存、队列相等，同probe根等价激活。RELEASED conditioned成功历史完成任务3594A，失败历史跳过失败137A；blind两历史均完成3594/3590A。PINNED conditioned成功历史失败3994A、失败历史跳过失败137A；blind均失败3994/3990A。上述为VM+非任务动作A；task成功另20A。正负训练与全部probe精确replay，所有版本安全。真实经验确实影响预设条件策略，但负历史面对改变环境也会损失任务，不是自发学习。尤其b0在skip后也能出现，不能把冷却耗尽错误解释成成功经验。
- [恢复修订原生3/3](https://github.com/iFrankStudio/OCEE/actions/runs/45319202821) @9681065a；[完整协议与实测](./experiments/E001/DEV-S29-RECOVERY-MEMORY-REVISION.md)。RELEASED两策略真Quota→GC276B→step4108 Ok/20A；STALE保留b1/v0、3379A；REVISE step4110真CAS b0/v1/3A、3415A（净+36A，根大小与索引成本不同，不是纯CAS费）。PINNED均失败/8Conflict/0B，STALE3778A、REVISE3747A，b1/v1。STALE_CAS实际研究者step4110付费3A写b1/v1，VM step4111旧b0 CAS真Conflict/0A，未覆盖、无重复写，任务保留且VM返回成功；non-task3412A不含外部3A。全snapshot/audit/峰值复演，B20不重复。index峰3694/3854B未计Quota。
- 两研究首次均通过，无红例，不等同全workspace回归。**下一项：把结果记忆与冷却状态分开，验证同一持续活动VM跨连续任务实际更新并应用，不把冷却清零当作成功。** 仍为研究者预设任务与策略，不宣称自发学习/演化。G0 GO/G1,G2 NO-GO；正式E001尚未校准/预注册。

### 2026-11-24 — 同一活动根连续任务、真实结果与重试资格分离（待验收）
- [新冻结协议](./experiments/E001/DEV-S29-CONTINUOUS-OUTCOME-AND-COOLDOWN.md) @2dd6c1f2，源 @8a0e9a1e，触发[原生CI #45320513669](https://github.com/iFrankStudio/OCEE/actions/runs/45320513669) @2066d50f，seed129真实P2 B20。旧b0既可能真正恢复又可能单纯冷却结束，新1字节三态：1=失败且待跳过、2=上次失败但已消耗冷却、0=真正恢复成功；仅真实GC后任务retry Ok才可正反馈，直接任务Ok不冒称维护成功。
- 单一付费安装活动根一次真实激活，CIL bounded three-slot loop、动态任务路径0/1/2、每项280B，真实付费Observe40次有限等待；全程同continuation，无研究者替它提交任务或写记忆，固定start+25唯一外部64B pin CAS（release/retain），190tick horizon。期望真实1/v0→2/v1→0/v2（release），或1/v0→2/v1→1/v2（pinned），先两项Quota，第三项可能真实GC后retry成功；记录真实task/index/GC/记忆时序、VM/action/external A、root字节和storage/index峰、完整snapshot/audit重放。不是长期连续生存、内生目标或策略学习。
- 新CI尚未验收；后续先查实时HEAD及两臂实际日志，保留失败。G0 GO/G1,G2 NO-GO，正式E001尚未校准/预注册。

### 2026-11-24 — 同一活动根连续三任务原生2/2验收
- [协议/实测](./experiments/E001/DEV-S29-CONTINUOUS-OUTCOME-AND-COOLDOWN.md)，[CI #45320513669](https://github.com/iFrankStudio/OCEE/actions/runs/45320513669) @2066d50f，2/2 success、原生每臂1 passed/0 failed。共同seed129真实P2 B20，root def0efb4…/19588B，一次真实外部激活，start3075/A11993。首任务Quota3075、Index3079、b1/v0@3089，固定研究者pin CAS3100/6A；第二任务Quota3132、b2/v1@3134（仍为失败历史），第三任务Quota3176、Index3180。RELEASED真实GC276B@3184、重试Ok@3185/20A、b0/v2@3187；PINNED无GC、b1/v2@3190。三任务成功数1/0，非任务A7166/7487，VM2879/3206，世界4313/4287（含任务/外部），物理峰147935/147931B，未计费index序列化峰均3618B；安装1233A单列。全程单根continuation、精确audit/snapshot重放、B20无重复，未宣称自发任务/自主策略发明或长期生命。无本轮全workspace结论。
- 另先冻结[真实注册工具连续计算效用协议](./experiments/E001/DEV-S29-CONTINUOUS-REGISTERED-TOOL-UTILITY.md) @ba6518a7，再实现 @c59ed138，触发[新CI #45320955049](https://github.com/iFrankStudio/OCEE/actions/runs/45320955049) @1e595f3e，三seed129/130/131×RELEASED/PINNED。任务不再仅写固定字节：每项由同一VM实际付费Read REGISTRY、Exec真实先前P2选择的工具、由实际输出构造280B（1B结果+279B合成padding），重试重新读取及执行，研究端逐实际EXEC与Write独立hidden_transform核验，正确计算和存储提交分开计数。候选目标与注册工具hash不编码进根。保持固定事件时序与安全门，6组尚未验收。G0 GO/G1,G2 NO-GO，正式E001未校准/预注册。

### 2026-11-24 — 真实注册工具连续计算与维护，原生6/6
- [协议与完整实测](./experiments/E001/DEV-S29-CONTINUOUS-REGISTERED-TOOL-UTILITY.md)，[CI #45320955049](https://github.com/iFrankStudio/OCEE/actions/runs/45320955049) @1e595f3e，6独立原生job全success，逐日志验收。三真实P2种子同根72f296de…/24252B（额外安装1524A），真实读取各自注册工具、实际Exec新输入，129 Len输出3/4/5、130 XorFold31/7/25、131 ByteAt first7/8/9，合计21次实际Exec含3重试全部正确。只在3个RELEASED臂第三任务成功真实存储（各20A），PINNED均失败；18个不同任务共3完成，不把正确计算等同提交成功。实际目标GC276/279/276B，注册工具与其余所有版本保留，三态记忆1→2→0或1，均2Index。RELEASED VM3367/3371/3279A、world4333/4319/4316A；PINNED VM3572/3575/3572A、world4302/4287/4285A，外部CAS6A均相同。执行及注册表费用单列见协议。index未计费序列化峰3616/3389/3389B；单根、完整精确复演、B20不重复。根没有目标或已注册工具hash，实际结果1B+研究者padding279B；输入和间隔仍由根预先规定。下一步移除固定输入与固定任务轮次，读取普通世界发布数据并决定实际任务处理；不宣称自主目标或开放演化。G0 GO/G1,G2 NO-GO；正式E001未校准/预注册。无全workspace结论。

### 2026-11-24 — 公共任务出现驱动的有界调度（待验收）
- [冻结协议](./experiments/E001/DEV-S29-PUBLIC-OFFER-DRIVEN-TASKS.md) @36857f67、实现 @d31a59ca、[CI #45324803189](https://github.com/iFrankStudio/OCEE/actions/runs/45324803189) @ae1753ce，seed129两tape ARRIVALS / DUPLICATE_ONLY。使用现有ACL公开对象/public/E001/transform（研究者模拟world发布，owner0，空examples、单字节ID和真实输入）；不添加内核能力、根不含具体输入或发布时间，移除40Observe固定任务间隔。VM真付费轮询，保留last attempted ID与原输入的continuation，识别不同ID、动态hex ID产物路径、读取实际P2注册工具并Exec；实际GC后按原ID/input重试。每机会32 poll、总96、最多3不同连续任务；不宣称任意重排ID全局去重。
- 两tape start+0发布a，+22实际付费释放pin，+28/+48发布新b/c或重复a（公共对象版本改变）；固定+150终点。公共发布是明确的研究者环境夹具直接更新公共对象且保守失效索引，非个体权限或生产事件API，记录预先schedule hash及每次发布state hash，完整重放。预期真实新任务可驱动后续冷却消耗与恢复，重复发布不得重复计算或虚构经验。审计独立计算、实际提交、轮询成本、维护、安全性与状态重放。**CI尚未验收**；保留失败，下一轮先实时读取。仍为预设任务解析和有限循环，非内生目标/学习。G0 GO/G1,G2 NO-GO；正式E001尚未校准/预注册。

### 2026-11-24 — 公共任务驱动首轮编译红，原协议重试
- [CI #45324803189](https://github.com/iFrankStudio/OCEE/actions/runs/45324803189) @ae1753ce 两臂0/2，均 E0433 测试schedule hash缺Sha256作用域，尚未执行实验。既有dev dependency sha2不变，仅补局部Digest/Sha256导入 @1703198d；原协议、输入、时序、资源与断言全部不变。触发[原生重试 #45325604541](https://github.com/iFrankStudio/OCEE/actions/runs/45325604541) @a81f560e，尚未验收。保留红例。G0 GO/G1,G2 NO-GO。

### 2026-11-24 — 同ID待完成任务的资源反馈重试（待验收）
- [独立冻结协议](./experiments/E001/DEV-S29-PENDING-OFFER-RESOURCE-RETRY.md) @760f706d、实现 @08f38889、[CI #45325877503](https://github.com/iFrankStudio/OCEE/actions/runs/45325877503) @d82c0af0；在上一原协议重试#45325604541仍运行时启动。seed129真P2、同一付费预装两个根LAST_ID/PENDING，等价激活后固定180tick。初次同a/[44,7,2]真实Quota、真干扫描b1/v0，始终不解绑任何版本；+28环境重复发布原a或同ID不同输入，SPACE/ID_CONFLICT物理capacity+512B，NO_SPACE不增。无actor余额编辑、无研究者任务/记忆写入。
- PENDING同ID先比较实际公开输入与continuation保存输入，真Read自己的结果对象，仅NotFound再付费读/sys/resource-status、真实free>=280才允许原任务重试；已有产物或输入冲突不重复执行。三环境每个与LAST_ID同物理初态对照，预测SPACE仅PENDING正确重试提交（不把直接成功误写成维护正反馈），另两环境均无额外Exec/GC。严格区分轮询、lookup、sensor、Exec、任务费用、安装和实际存储/index峰，完整snapshot/audit与环境发布hash复演。仍仅同ID有限continuation语义，不宣称全局去重或跨重启持久队列。
- **两个新run仍未验收**，后续先读#45325604541和#45325877503实际日志及实时HEAD，保留红例，不改变冻结事件时序/阈值。G0 GO/G1,G2 NO-GO；正式E001未校准/预注册。

### 2026-11-24 — 公共任务驱动2/2与同ID资源重试3/3验收
- [公共驱动协议与实测](./experiments/E001/DEV-S29-PUBLIC-OFFER-DRIVEN-TASKS.md)，[重试CI #45325604541](https://github.com/iFrankStudio/OCEE/actions/runs/45325604541) @a81f560e **2/2原生**，首编译红#45324803189保留。seed129同根23f29368…/29283B，安装1839A；start3075，外部发布a/b/c或重复a @3075/3103/3123，解除pin@3097/6A。ARRIVALS真24公开Read、4Exec输出3/4/5/5，任务前三次Quota，真GC276B后3137重试Ok20A，b1→2→0；DUPLICATE_ONLY真33Read但仅1Exec/1Quota/1Index，b1不变，0GC。VM4578/3504A，world4298/2169A（含任务/外部），公开读取48/66A。安全版本、B20、审计/快照/事件重放通过。仅相邻ID去重。
- [同ID重试协议与实测](./experiments/E001/DEV-S29-PENDING-OFFER-RESOURCE-RETRY.md)，[CI #45325877503](https://github.com/iFrankStudio/OCEE/actions/runs/45325877503) @d82c0af0 **3/3原生**（六个配对策略轨迹）。共同付费安装LAST_ID与PENDING根（29283/32089B，安装3853A），相同start3075/A11993、首写3078 Quota、真实b1/v0@3092。SPACE +512B后PENDING真读NotFound3104、free706@3105（前194）、Exec3107、写Ok3108/20A，旧策略放弃；其后32次产物读取Ok/6A但不重复执行。NO_SPACE均不重试，ID_CONFLICT变更输入均拒绝额外执行。旧策略非任务A均6174，新策略SPACE9830/NO_SPACE9394/ID_CONFLICT6726，额外3656/3220/552A；无任何GC、b1/v0真实失败维护记忆不改，所有版本与B20保持、精确重放。index峰3852B未计费，成功产物物理峰190059B，其余189779B。
- **明确下一突破口**：状态识别与轮询代价过大；产物仅路径含ID，未持久绑定输入，last-ID仅在当前continuation有效。先建立真实提交的内容绑定收据，验证非相邻重复ID与重启后不再执行已完成任务、不同输入同ID拒绝；再利用实际成功/读取收据在continuation缓存，避免重复查已完成对象。不得从本实验声称全局去重/一般队列或自发目标。G0 GO/G1,G2 NO-GO；正式E001未校准/预注册，无本轮全workspace结论。

### 2026-11-24 — 内容绑定持久任务收据与连续状态缓存（待验收）
- [独立冻结协议](./experiments/E001/DEV-S29-CONTENT-BOUND-OFFER-RECEIPTS.md) @bf26bd85、源码 @cbafcd06、[CI #45331711663](https://github.com/iFrankStudio/OCEE/actions/runs/45331711663) @e0562933。seed129真实P2 B20；同一物理库存共同付费装VERIFY/CACHE根，180tick三臂SPACE、REORDER、ID_CONFLICT；两根都将实际输出1B+输入SHA256 32B+247B合成padding作为280B真实create-CAS产物，ID路径不变。每次新见ID先实际Read产物，只有NotFound才执行；已有产物检查长度/输入摘要，匹配或冲突均不重做/覆盖。0待完成/1真实提交或已核验/2内容冲突；实际Write Ok后状态1，返回值4*实际尝试次数+状态，不等于成功数。CACHE仅在同一continuation已验证相同ID/input时跳过重复查询；VERIFY仍付费验证。缓存仅适用此单writer无外部修改receipt的夹具，不声称一般并发一致性。
- SPACE按原+28空间恢复条件测试相同实际成功、b1/v0不变，冻结预测CACHE成功后少查产物、总非任务A较低。REORDER/ID_CONFLICT初始空闲1200B，+0/+16/+32发布a/b/a（最后a原输入或变化），预期只实际完成a,b，识别再次a为匹配或冲突。自然grant后一次显式研究者同根重启，保留已提交对象且空continuation；新阶段+0/+16/+32发布a/c/a，预期只真正完成c并重新付费验证a。两阶段各+180固定终点，非相邻去重与重启均需实际读取和独立核验原产物、状态码、原生Exec、费用、全精确replay。未改变内核权限。新CI尚未验收，保留任何红例并从实时HEAD继续。G0 GO/G1,G2 NO-GO；正式E001尚未校准/预注册。


### 2026-11-24 — 内容绑定收据的算力耗尽反例与有界轮询独立修订（待验收）
- 原冻结协议 [#45331711663](https://github.com/iFrankStudio/OCEE/actions/runs/45331711663) 0/3，原断言 last_return=None。只读诊断 [#45332565092](https://github.com/iFrankStudio/OCEE/actions/runs/45332565092) @a5e8c889 同为0/3，均仅运行VERIFY首阶段，CACHE未到达。初始11993A；SPACE终42A、102次Gas、VM9041/world2910A，27公开读/27收据读、真实2次Exec/Write、1索引；REORDER/ID_CONFLICT终8A、95次Gas、VM11633/world352A，40公开读/39收据读、2次真实成功写入。VM未完成，不得将局部写入冒称协议通过。反复核验280B收据每对读约306A。
- 冻结独立[有界轮询预算协议](./experiments/E001/DEV-S29-BOUNDED-RECEIPT-POLL-BUDGET.md) @6a1c7348，源码/工作流 @edb9793a， [原生CI #45334665847](https://github.com/iFrankStudio/OCEE/actions/runs/45334665847) 运行中。v2仅将两策略轮询32/96降至20/60，180tick、原自然A grant、事件时序、数据、权限与所有验收判据不变；三臂配对。另独立运行原32/96 CACHE-only SPACE，同原两根付费安装、同单臂断言，不拿未完成VERIFY作成功对照。保留所有红例，不能声称20最优、缓存一般并发安全或个体自主学习。后续优先读取新run真实日志和最新HEAD。G0 GO/G1,G2 NO-GO；正式E001未校准/预注册。


### 2026-11-24 — 有界轮询SPACE初步配对成功与通用局部观察实验（部分待验收）
- [有界轮询CI #45334665847](https://github.com/iFrankStudio/OCEE/actions/runs/45334665847) @edb9793a，SPACE job 138243808799 已原生通过，其他3 job仍运行，不能冒称4/4。共同base f4068a45…，11993A/start4099，两策略均实际Quota@4103、Ok/20A@4130、2Exec、b1/v0、terminal9，原安全与精确复演判据通过。VERIFY真实20次成功收据读/120A，CACHE 0次/0A（均4次NotFound），公开读24次/48A相同；VM8237/3746A，world2886/2761A，非任务A11103/6487，节省4616A/41.57%。两根37106/37461B，共同付费安装4678A，物理峰203254B，index未计费峰3852B，B20不重算。只适用该单writer固定tape和20/60上限。
- 独立[通用付费自身对象前缀观察协议](./experiments/E001/DEV-S29-PAID-OWN-PREFIX-OBSERVATION.md) @cce68dd7 先冻结，源码 @b1322980、[CI #45334944941](https://github.com/iFrankStudio/OCEE/actions/runs/45334944941) 运行中。新增既有Read路径 /sys/self-prefix/<n>/<relative>，canonical 1..256、自有ACL，返回真实物理长度LE4与实际前缀，收费2+ceil(payload/64)；无新VM原语、无收据oracle、无免费hash。三臂用原32/96与原自然供给，读取33B实际前缀+4B长度，由VM检查真实280B长度和SHA256；实际完整产物、真实重复ID/重启、费用、原判据仍保留。独立内核契约测试+三原生实验job。待实际结果；新能力是研究者提供，不宣称内生发明或开放式演化。继续先核查两个CI及实时HEAD。G0 GO/G1,G2 NO-GO；正式E001未校准/预注册。


### 2026-11-24 — 持久收据与低成本局部感知原生8/8验收
- [有界轮询#45334665847](https://github.com/iFrankStudio/OCEE/actions/runs/45334665847) @edb9793a 全4 job成功。SPACE已记录；REORDER与ID_CONFLICT两阶段VERIFY/CACHE非任务A8550/3150、8923/3445，真实成功收据读26/1、28/2，首阶段仅a,b成功写，重启后仅c成功写，终态状态码9/5或10/6。原上限32/96独立CACHE SPACE也成功（7387A，0成功收据读，原真实Quota→Ok），不抹去原VERIFY红例。
- [局部读取#45334944941](https://github.com/iFrankStudio/OCEE/actions/runs/45334944941) @b1322980 全4 job成功，原32/96上限不变；内核契约真实280B/6A vs 37B/3A，权限/边界/低A/replay检查通过。SPACE VERIFY/CACHE非任务10664/7404A，成功读32/0，同样真实完成a；REORDER/ID_CONFLICT第一阶段7324/3925A，第二阶段7425/4089A，实际成功收据读38/1和40/2，真实正确产物、同ID冲突、跨重启判据均通过。根37666/38021B，共同安装4748A，全部精确复演，B20不重算，物理峰204374/204653/204933B，未计费index峰SPACE3852B。缓存仍限定单writer、不外部修改产物。尚无全workspace回归或内生策略发明证据。下一步应检验真实信息感知成本与延迟/漏任务的取舍，不以固定tape上最省A等同一般最优。G0 GO/G1,G2 NO-GO；正式E001尚未校准/预注册。


### 2026-11-24 — 真实反馈驱动的感知节奏与机会成本（待验收）
- 先冻结[配对感知协议](./experiments/E001/DEV-S29-FEEDBACK-PACED-PERCEPTION.md) @9ec81d2f，源/工作流 @cedaa628，[原生CI #45336375358](https://github.com/iFrankStudio/OCEE/actions/runs/45336375358) 已触发。DENSE复用已通过的prefix CACHE 32/96，PACED仅在VM真实见到相同ID/input且已验证状态1/2时真付费Observe三次再poll，单scope/aggregate改12/36；pending分支不延迟。不新增内核/VM原语，不由host控制个体行动，不称自主学会阈值。
- seed129真实P2 B20、共同付费安装两个root、同1200B空闲、自然A grant、等价激活、120tick。三预冻结tape都+0发布a：STABLE +16发布持续b，预测两者完成a,b且PACED少花非任务A；PULSE +7发布b/+9恢复a，预测PACED漏b而DENSE观测并完成；LATE +40发布持续b，预测DENSE已结束而PACED仍能观测。真实独立计算、内容绑定产物、没有重复执行/写入、正常结束、实际费用、感知与完成时间、安装/峰值、完整snapshot/audit及环境publication hash复演均需验收。比较的是轮询数量与间隔联合策略，不是隔离单因素，也非一般最优。CI尚未验收，继续先读真实日志并保留红例。G0 GO/G1,G2 NO-GO；正式E001尚未校准/预注册。


### 2026-11-24 — 等感知频率的公共数据精确记忆（待验收）
- 在感知节奏#45336375358仍运行时，独立冻结[精确公共payload记忆协议](./experiments/E001/DEV-S29-PUBLIC-PAYLOAD-MEMO.md) @db69c785，源码/工作流 @03a08fe2，[CI #45336719723](https://github.com/iFrankStudio/OCEE/actions/runs/45336719723) 已触发。两策略均复用已通过prefix CACHE 32/96，MEMO只在真实付费公开Read的payload与continuation上一份真实payload字节完全相等且当前收据状态非0时跳过重复字段解析；状态0继续原pending路径，环境数据变化继续完整解析。没有内核权限/VM原语变更，使用已有有界EqBytes及原计费模型，不宣称一般CPU成本。
- 原SPACE/REORDER/ID_CONFLICT三tape、真实seed129/P2、共同付费安装、原自然grant/180tick与所有原有收据/安全/重启/复演判据不变。新增严格配对断言：每阶段两策略实际世界动作序列（step/action/code/payload/world fee，包括空闲步）完全相同，且MEMO非任务A严格更少。避免将减少感知次数带来的机会损失与纯计算节省混淆。两根均为研究者编写，无自主学习或一般并发缓存保证。两个CI尚未验收，继续先核查实时HEAD及实际日志。G0 GO/G1,G2 NO-GO；正式E001未校准/预注册。


### 2026-11-24 — 感知节奏3/3验收、精确记忆2/3及资源反馈严格核算修订
- [感知节奏#45336375358](https://github.com/iFrankStudio/OCEE/actions/runs/45336375358) @cedaa628 全3原生通过。共同base dce097d0…，11993A/start4099；DENSE/PACED根38021/39434B、共同安装4858A。STABLE 真实均完成a,b，非任务3810/1845A（节省1965A/51.57%），公开读45/17，Observe0/45，b延迟4/5tick；PULSE DENSE真见b@4106且4110完成，PACED漏b（仅a），非任务3250/1306A；LATE DENSE终4137早于b@4139，PACED真见4140/完成4144，非任务2698/2349A。各原判据、B20、无Gas、完整snapshot/audit与环境hash复演通过。不同任务数不能直接比较单位效率，非一般最优。
- [精确公共记忆#45336719723](https://github.com/iFrankStudio/OCEE/actions/runs/45336719723) @03a08fe2 **2/3原生**。REORDER/ID_CONFLICT两阶段均原严格世界timeline通过，非任务3925→3218、4089→3320A，VM3822→3115、3982→3213，world143/127A完全相同，根38021/41202B（+3181B）。SPACE红，严格raw timeline差异仅真实余额传感（4105/4121/4124/4127）、Observe4118及索引4107（MEMO168750B/2653A vs base171931B/2703A）；动作/时刻/code一致。原失败保留，不宣称3/3。
- 独立冻结[资源核算配对协议](./experiments/E001/DEV-S29-PUBLIC-MEMO-RESOURCE-ACCOUNTING.md) @0b7259b2，源码/工作流 @22eafd69，[CI #45338715681](https://github.com/iFrankStudio/OCEE/actions/runs/45338715681)。SPACE原两根/世界/事件/供给/所有安全和任务判据完全不变；仅新配对验收分别从真实初始余额与累计VM+world费用重建每次读/Observe的A，原字段其余完全相等，索引独立检查真实计费及scan+active root B相等，真实index费用计入净A；仍要求所有其他payload、fee、action、code、step严格相等、精确replay和净A严格下降。待验收，继续读取实际日志；原严格失败不可覆盖。G0 GO/G1,G2 NO-GO；E001正式未校准/预注册。


### 2026-11-24 — 资源核算1/1验收与有限反馈自适应感知（待验收）
- [资源核算#45338715681](https://github.com/iFrankStudio/OCEE/actions/runs/45338715681) @22eafd69 **1/1原生**。SPACE共同base f5969ad7…，11993A/start4099，根38021/41202B，真实Quota4103→Ok4130，2Exec，终态9，4次NotFound，公开读36次/72A，无成功收据读。BASE/MEMO VM4630/4104A，world2849/2799A，非任务7459/6883A（节省576A，7.72%），最终4514/5090A。索引实测171931/168750B，2703/2653A，14版本，scan+active root=209952B均等；传感余额严格重建，其他世界动作时刻/结果完全相等，完整snapshot/audit复演。共同安装4969A，物理峰207910B，未计费index3852B，B20。原严格memo矩阵2/3红例保留，独立修订通过；不宣称原3/3。
- [有限反馈自适应感知协议](./experiments/E001/DEV-S29-ADAPTIVE-PERCEPTION-BACKOFF.md) @9f86656，源码/工作流 @a3e54ea，[CI #45342221229](https://github.com/iFrankStudio/OCEE/actions/runs/45342221229)。同物理付费装DENSE/PACED/ADAPT三个root，真实seed129/P2、原自然供给、1200B空闲、120tick。ADAPT在真实fresh receipt lookup清零有界重复计数，前两次确认相同输入/ID快速读，第三次及以后以3次真实付费Observe延迟再读，12/36上限，pending不延迟；没有host动作选择/内核新能力。原STABLE/PULSE/LATE与新SHIFTED_PULSE（+24 b/+26 a）共四tape。预测ADAPT前三组都能完成2任务但shifted漏b，而DENSE/PACED在shifted均完成，明确非支配反例。真实任务、费用、时序、安全、返回、完整复演均需验收，若失败保留原预测。固定计数规则由研究者给定，非自主学习、持久策略经验或开放演化。G0 GO/G1,G2 NO-GO；正式E001尚未校准/预注册。


### 2026-11-24 — 自适应退避4/4验收与跨重启持久经验对照（待验收）
- [自适应感知#45342221229](https://github.com/iFrankStudio/OCEE/actions/runs/45342221229) @a3e54ea **4/4原生**，12条轨迹。DENSE/PACED/ADAPT根38021/39434/42510B，common base dbfbc330…，共同安装7523A，start4099/A11993。STABLE实际完成2/2/2，非任务3810/1845/2043A；PULSE 2/1/2，3250/1306/1886A；LATE 1/2/2，2698/2349/2565A；SHIFTED_PULSE 2/2/1，4525/2119/1332A。ADAPT前三种捕获b、shifted漏b，明确非支配；真实初读与等待轨迹、正确产物、无Gas、原B20、原全部安全及snapshot/audit复演通过。物理峰248715/248995B，不称一般最优或学习规则。
- [跨重启持久经验协议](./experiments/E001/DEV-S29-DURABLE-PERCEPTION-EXPERIENCE.md) @7342ed8，源码/工作流 @ed7e0c4，[CI #45345775386](https://github.com/iFrankStudio/OCEE/actions/runs/45345775386)。两根共同真实付费安装，原P2 seed129/B20/自然A/free1200B，20/60轮询、120tick每阶段。同训练tape a/+7 b/+9 a，真实成功写入累计2次才真实create-only写1B经验；两根都付费读取相同记录，仅USE重启后根据真实Ok及b1走32阈值高频，IGNORE仍2阈值。自然grant后同根等价重启，测试新c/+9 d/+11 c，预测USE额外真实完成d。无研究者写入经验、无内核/VM新权限。真实任务、全费用、marker v0、独立产物验证、完整复演，若失败不移动冻结tape。仍是研究者定义的经验映射而非自行发现阈值/优化目标，不宣称可推断未观察变化。G0 GO/G1,G2 NO-GO；正式E001未校准/预注册。


### 2026-11-24 — 持久经验正迁移1/1验收、负迁移与自然无经验对照（待验收）
- [持久经验#45345775386](https://github.com/iFrankStudio/OCEE/actions/runs/45345775386) @ed7e0c4 **1/1原生**，4条两阶段轨迹。IGNORE/USE根50739/51620B，共同安装6415A；训练start4099/A11993，均实际a@4104,b@4110 commit，marker@4183真实create v0 b1/3A，终4185，非任务2542/2602A。自然grant与同根重启start5123/A11993，均实际Read marker Ok/2A；IGNORE仅c@5128、错失d，USE见d@5132/commit5136、无Observe，非任务2132/2748A（额外616A），实际终5204/5159，返回5/9。独立产物、版本保留、无Gas、B20、精确snapshot/audit及环境hash复演通过，物理峰231326B训练、231606/231886B测试，未计费index0。只证明预设经验映射下有因果行为变化与指定环境任务收益，不等于学习规则/一般正迁移。
- [经验迁移边界协议](./experiments/E001/DEV-S29-EXPERIENCE-TRANSFER-LIMITS.md) @55da6bd，源码/工作流 @336e2ba，[CI #45353820725](https://github.com/iFrankStudio/OCEE/actions/runs/45353820725)。复用已验收两根，原种子、自然预算、物理安装、每阶段120tick与全部安全/复演判据不变。NEGATIVE_TRANSFER 同原训练获得真实经验，测试c/+40持续d，预测IGNORE完成两项而USE过早结束漏d；ABSENT_EXPERIENCE 仅a训练自然不足两次成功不写marker，测试原c/+9 d/+11 c，两根真实NotFound、均只完成c，严格世界action timeline等价（VM费用可异）。不由研究者删/造经验，不变根、不预设优化收益，保留全部反例。后续先验收原生结果，再考虑可逆的策略评价；G0 GO/G1,G2 NO-GO，正式E001尚未校准/预注册。


### 2026-11-24 — 经验迁移边界0/2原生反例及独立核算补充（待验收）
- [原迁移边界#45353820725](https://github.com/iFrankStudio/OCEE/actions/runs/45353820725) @336e2ba **0/2**，均编译并实际运行。NEGATIVE_TRANSFER IGNORE训练通过，测试真实c@5128,d@5167成功、非任务3316A、无Gas、完整复演，但+120尚未结束（last_return9不能替代finished），USE未运行。不能宣称负迁移已证实。ABSENT_EXPERIENCE两根均训练仅a、无marker，测试仅c@5128，真实动作时刻/code/fee一致，非任务2115/2175A、最终9858/9798A、同结束5204；原严格timeline失败，逐项比对54个差异均仅Observe返回余额，首次5132余额11403/11394。原红例保留。
- 独立冻结[迁移边界核算协议](./experiments/E001/DEV-S29-EXPERIENCE-TRANSFER-ACCOUNTED.md) @0e9257a，源码/工作流 @f06b869，[CI #45364104020](https://github.com/iFrankStudio/OCEE/actions/runs/45364104020)。NEGATIVE_TRANSFER仅测试horizon120→180，原训练120、事件时序/两根/自然资源/所有任务与安全/正常结束/replay判据不变；ABSENT_EXPERIENCE原120不变，成功Observe逐次从实际初始A减全部累计VM+world费用重建返回余额，费用1、step与其余字段严格相同，终余额独立核验，其他action/code/fee/payload完全相等。既不改root也不人工注入记忆，保留原协议0/2；等待真实新CI。下一步待证据后再推进可逆评价，G0 GO/G1,G2 NO-GO；正式E001未校准/预注册。


### 2026-11-24 — 经验负迁移补充2/2验收与付费撤销经验（待验收）
- [补充#45364104020](https://github.com/iFrankStudio/OCEE/actions/runs/45364104020) @f06b869 **2/2原生**。NEGATIVE_TRANSFER测试180tick，IGNORE完成c@5128,d@5167，真实终5243（正好+120），USE只完成c@5128，终5150早于d@5163；非任务3316/2084A，最终8637/9889A，真实读marker均2A。原120tick失败保留。ABSENT_EXPERIENCE保持120tick，两根训练仅a、无marker，测试仅c@5128、终5204，实际54次Observe余额分别严格重建，其他动作/时刻/fee/payload相等，非任务2115/2175A、最终9858/9798A。两根50739/51620B，共同安装6415A，峰231045–231886B，B20、正确产物、精确复演。经验既可正迁移也可负迁移；无经验不产生策略差异（但VM开销不同）。
- [付费经验撤销协议](./experiments/E001/DEV-S29-DURABLE-EXPERIENCE-REVISION.md) @5eca47e，源码/工作流 @20085dc，[CI #45387874278](https://github.com/iFrankStudio/OCEE/actions/runs/45387874278)。两根均派生原USE，20/60轮询、seed129 P2/B20、共同真实付费安装、free2000B、自然grant、同根等价重启，三阶段180tick。REVISE仅在真实加载marker1且本阶段实际成功提交恰1项时发出CAS0写0，KEEP不撤销；0与缺失区分，后续实际Read0恢复默认自适应，单writer/v0一次性撤销，无重新启用能力。三臂RECOVERY/REVERSAL_COST/RETAIN共18阶段轨迹，分别测试恢复延迟任务、撤销后错失短暂任务、两成功时保留经验。冻结任务tape，无外部经验写入，无对未观测任务的oracle。成功数只是有限代理指标，规则研究者提供，不称自主发明、一般学习或最优。保留红例，待实际CI验收。G0 GO/G1,G2 NO-GO；正式E001未校准/预注册。


### 2026-11-24 — 付费经验撤销3/3验收与真实版本双向修订（待验收）
- [经验撤销#45387874278](https://github.com/iFrankStudio/OCEE/actions/runs/45387874278) @20085dc **3/3原生**，18条阶段轨迹。KEEP/REVISE根52531/54283B，base75989d97…，共同安装6693A。训练均真实a@4104,b@4110、marker1/v0@4183/3A、非任务2602A，终4185。RECOVERY/REVERSAL_COST反馈阶段均仅c@5128，KEEP保留1，REVISE实际CAS0写0@5149/3A（v1），非任务2084/2102A、终5150/5151。RECOVERY测试KEEP仅e，REVISE真实额外f@6191，非任务2084/3482A、终6174/6267；REVERSAL_COST测试KEEP完成e,f@6152/6160、REVISE漏f，非任务2751/2211A。RETAIN两根反馈/测试均2项真实提交、无CAS修改、非任务2748/2751A均等。真实marker读取、版本、任务正确性、所有安全、无Gas、B20和精确复演通过；峰235781–236901B。证明有限规则能修订经验，也会错误撤销，不是学习规则或普适收益。
- [双向版本化经验修订协议](./experiments/E001/DEV-S29-REVERSIBLE-EXPERIENCE-CAS.md) @1ac1d06，源码/工作流 @f045d14，[CI #45389246393](https://github.com/iFrankStudio/OCEE/actions/runs/45389246393)。复用现有/sys/self-object/元数据真实bytes+version收费读取，两个root从真实version建立CAS、U32 MAX保留值拒绝修改、真实NotFound才create；不新增内核权限。ONE_WAY和REVERSIBLE都在加载1且一次真实提交时撤销，后者加载0且两次真实提交时重新启用。seed129/P2/B20、共同付费安装、free3000B、20/60预算、五阶段180tick、等价同根重启、固定a..j环境tape。预测双方1/v0→0/v1，REVERSIBLE再1/v2并捕获短暂h，最后错失延迟j再次0/v3；ONE_WAY持续0/v1，漏h但捕获j。逐次真实CAS/版本/费用、任务独立正确性/保留、精确复演等均需验收；无外部经验写入/未观测机会oracle，无新能力权限，无自主发现规则。保留红例，G0 GO/G1,G2 NO-GO；正式E001未校准/预注册。


### 2026-11-24 — 双向经验修订编译失败与纯重复定义修复
- [原双向CAS #45389246393](https://github.com/iFrankStudio/OCEE/actions/runs/45389246393) @f045d14 0/1，rustc E0428：EXPERIENCE_META/repeated_experience_root完全重复定义（7518/7519与7751/7752），未实际执行实验。已确认两个builder文本逐字相同，仅删除尾部重复副本6647字符，保留首个builder及完整冻结测试；调整workflow step名称重触发，源码 @ec7e58c，[CI #45390225834](https://github.com/iFrankStudio/OCEE/actions/runs/45390225834)。未更改策略、事件、资源、判据、预期或世界内核；原编译红保留。先读新CI真实结果，未验收不能将双向修订视为已实现能力。G0 GO/G1,G2 NO-GO。


### 2026-11-24 — 独立观察不可区分性反例（待验收）
- 双向CAS重跑[45390225834](https://github.com/iFrankStudio/OCEE/actions/runs/45390225834) @ec7e58c 当前in_progress / 未验收；原重复定义编译红保留。
- 独立冻结[观察不可区分性协议](./experiments/E001/DEV-S29-OBSERVATION-ALIASING.md) @11368f7，源码/工作流 @23d7779，[CI #45391665473](https://github.com/iFrankStudio/OCEE/actions/runs/45391665473)。仅复用已验收REVISE根（非待验收双向根），seed129/P2/B20、付费安装、free2000B、自然grant、原训练a/+7 b/+9 a实际成功及创建1/v0。随后同一真实重启快照克隆成SILENT（只有c）与LATE（c/+40 d持续）两臂，固定180tick。预测两臂实际动作及全部ActionResult（包括VM/world费、payload）逐项完全相等、都仅成功c且自行CAS0→0/v1、终止早于+40、私有状态与资源账户相等；但外界public发布1/2次、最终公共对象内容/版本和世界摘要不同。分别完整snapshot/audit/publication复演，真实任务正确性、保留、费用、无Gas和B20严格验收。若通过，是单次观察轨迹不可区分的反例，说明成功数不足以识别未观测机会，不等于证明所有学习不可能。无oracle、无root/内核修改。待实测后再研究付费探索与评价，G0 GO/G1,G2 NO-GO。


### 2026-11-24 — 双向经验CAS与观察不可区分性验收、付费延迟探索（待验收）
- [双向CAS重跑#45390225834](https://github.com/iFrankStudio/OCEE/actions/runs/45390225834) @ec7e58c **1/1原生**，10条阶段轨迹。ONE_WAY/REVERSIBLE根58684/59699B，共同安装7416A，base237bc8e2…，各阶段start4099/5123/6147/7171/8195、11993A。均真实创建1/v0@4183，CAS0写0/v1@5149；REVERSIBLE真实读取v1后CAS1写1/v2@6266，再真实读取v2后CAS2写0/v3@8221，各写3A、存在记录原子元数据读3A。阶段任务数ONE_WAY 2/1/2/1/2，REVERSIBLE 2/1/2/2/1，总数均8；非任务分别2617/2130/3502/2168/3406及2617/2130/3516/2708/2067A。REVERSIBLE捕获h@7184而ONE_WAY漏h；最后ONE_WAY捕获j@8239而REVERSIBLE漏j。无Gas/B20/真实版本与费用、正确产物、全部安全和精确复演通过，物理峰247350–249030B。原编译红保留。双向规则并非普适优化或自主发现评价。
- [观察不可区分性#45391665473](https://github.com/iFrankStudio/OCEE/actions/runs/45391665473) @23d7779 **1/1原生**。同训练所得真实restart、root a0d701df…54283B、安装3401A、start5123/A11993，两臂公共发布1/2次，实际均20次Read（5124及5129–5148）、一Exec、一c提交、一次真实撤销、终5151，VM2050/world72/非任务2102A、final9871A、峰183466B。全部ActionResult及动作时刻/私有状态/节点资源完全相同，公共最终版本/内容与世界摘要不同；各自精确复演。明确单轨迹观察混叠，不能从未观察事件推断机会损失。
- [付费延迟探索协议](./experiments/E001/DEV-S29-PAID-DELAYED-EXPLORATION.md) @d6226ca，源码/工作流 @dec1d1d，[CI #45393683463](https://github.com/iFrankStudio/OCEE/actions/runs/45393683463)。基于已验收单向REVISE，PROBE仅加载1且连续重复计数10时插入24个真实付费Observe后原increment；保留20/60上限与原经验评价规则。共同付费安装、seed129/P2/B20/free2000B/自然grant、同根重启、180tick。三臂SILENT仅c（预测同一提交但额外成本）、LATE c/+40持续d（预测探索捕获d且保留经验）、MID_PULSE c/+21 d/+23 c（预测原策略捕获、探索漏失且撤销）。真实费用、安全、版本、正确产物、精确复演等须原生验收；无oracle、未改变内核能力，固定探索规则非学习。保留反例，G0 GO/G1,G2 NO-GO；正式E001未校准/预注册。


### 2026-11-24 — 延迟探索0/3静态验证拒绝与有界循环补充
- [原探索#45393683463](https://github.com/iFrankStudio/OCEE/actions/runs/45393683463) @dec1d1d **0/3**，均正常编译，但PROBE在Program::validate处InvalidProgram("block count")（7913），展开24个Observe/feedback共48块超出既有VM限制；未开始实验轨迹，无行为/收益结论。原红保留，不放宽验证器。
- [独立有界探索协议](./experiments/E001/DEV-S29-PAID-DELAYED-EXPLORATION-BOUNDED.md) @59c4ac3，源码/工作流 @7d51a01，[CI #45394634684](https://github.com/iFrankStudio/OCEE/actions/runs/45394634684)。复用既有RepeatScoped scope99、local24/aggregate72，真实付费Observe与反馈循环完成后原increment；原count10触发/20/60轮询/BASE不变。每机会scope最多一次完整等待，若同scope内新receipt身份出现不能重置等待计数，较原每身份run展开更窄，不称普遍等价修复。冻结原环境tape、seed、预算、180tick、预期、原任务/费用/安全/复演判据。保留原无效根及原测试，新独立workflow。无VM/世界权限更改。G0 GO/G1,G2 NO-GO；待实际CI。


### 2026-11-24 — 有界探索3/3验收与资源感知经验评价（待验收）
- [有界探索#45394634684](https://github.com/iFrankStudio/OCEE/actions/runs/45394634684) @7d51a01 **3/3原生**，12阶段轨迹。BASE/PROBE根54283/55764B，共同付费安装6895A，base76fb2558…，训练均a@4104,b@4110、marker1/v0@4183、非任务2602A，测试start5123/A11993。SILENT均c@5128、真实撤销0/v1，非任务2102/2256A（额外154A=130VM+24world），终5151/5175。LATE BASE仅c且撤销，PROBE额外d@5168（延迟5）保留1/v0，非任务2102/3618A、终5151/5214。MID_PULSE BASE捕获d@5148并保留，PROBE仅c且撤销，非任务3792/2256A、终5171/5175。全实际费用、正确产物、marker版本、B20、无Gas/维护、精确复演通过；峰239014–239574B。原图块数拒绝保留；有限探索有正、负及纯成本反例，非普适收益。
- [资源感知经验评价协议](./experiments/E001/DEV-S29-RESOURCE-AWARE-EXPERIENCE-EVALUATION.md) @bd4cf94，源码/工作流 @b3ff313，[CI #45396685822](https://github.com/iFrankStudio/OCEE/actions/runs/45396685822)。两根均基于已验收PROBE，在实际加载1且本阶段两次真实提交后付费Read /sys/resource-status；COUNT忽略，FLOOR使用真实a_available <9000触发既有CAS0写0，非Ok保留。原创建、单提交撤销、0默认节奏不变，9000研究者定义储备偏好而非学得价值。seed129/P2/B20、共同付费安装、free2000B、自然grant、同根重启、三阶段180tick。RESERVE_LATE预测晚任务两次均捕获、FLOOR在反馈撤销后测试节省非任务A；RESERVE_PULSE预测撤销后漏短暂f；RETAIN_EARLY预测早期低成本任务两组保留。要求逐次实际余额重建、真实资源报告容量/用量、费用2A、全部任务/版本/安全/复演；无新世界权限、外部经验写入或未观测事件oracle，保留新反例。G0 GO/G1,G2 NO-GO；正式E001未校准/预注册。


### 2026-11-24 — 已验收探索轨迹的实际成本价值边界（独立离线诊断）
- [价值边界](./experiments/E001/DEV-S29-EXPLORATION-UTILITY-FRONTIER.md)，从#45394634684三个原生job逐条提取6条测试阶段日志，独立核对真实VM+world/起止余额/任务费/非任务费，任务数由真实提交tuple计数。SILENT/LATE/MID_PULSE PROBE-BASE总费+154/+1536/-1556A，任务差0/+1/-1；实际Observe次数24/48/24（LATE两个完整等待）。假设外部分析值V A/提交，净差-154、V-1536、1556-V；等权三tape总任务数相同，BASE平均省134/3 A，1536<V<1556才同时有利于两个非静默tape，V1546时假设静默比例<5/82才有利。该分析不赋予真实奖励、不推断环境分布、未计独立安装/时间/未来记忆代价，非新增科学实验。已提交可复算JSON/Python，实际日志核算通过；当前资源评价#45396685822尚未验收。后续优先研究真实资源回报闭环而非继续手工阈值扩张，需独立冻结经济契约，保持既有实验不变。G0 GO/G1,G2 NO-GO。


### 2026-11-24 — 资源感知经验评价3/3验收与有限任务资源结算（待验收）
- [资源评价#45396685822](https://github.com/iFrankStudio/OCEE/actions/runs/45396685822) @b3ff313 **3/3原生**，18阶段轨迹。COUNT/FLOOR根56906/57893B，共同安装7192A，base fd8afe75…；训练均a,b、marker1/v0、非任务2602A。RESERVE_LATE/RESERVE_PULSE反馈均c,d成功，真实资源读取8330A@5213/2A，FLOOR CAS0写0/v1@5214/3A，非任务3628/3695A。RESERVE_LATE测试两根均e,f，非任务3691/3485A（节省206A，反馈额外67A）；RESERVE_PULSE测试COUNT两项、FLOOR仅e，非任务2921/2211A；RETAIN_EARLY反馈/测试均两项、保留1/v0，实测读9040/9037A，非任务2918/2973及2921/2976A，评价开销55A。注意FLOOR最终余额8980/8977A低于9000是读后VM开销，规则只检查真实观察时点，不保证最终储备。真实余额/容量/用量核算、任务及版本、安全、精确复演、无Gas/B20通过；峰243766–244886B。有限人工资源阈值可改变行为，不是内生价值或普适效率。
- [有限任务托管契约](./experiments/E001/DEV-S29-FINITE-TRANSFORM-ESCROW.md) @025c1cc，源码/工作流 @7b440dc，[CI #45407022797](https://github.com/iFrankStudio/OCEE/actions/runs/45407022797)。独立默认关闭的世界开发机制，可信构造时显式独立有限payer endowment<=4096A，单一不可变公开任务（空examples、单小写ID、1..255B输入）、固定正价，只有真实普通create-only Write成功、精确绑定任务且独立校验正确输出/输入SHA256/填充/version0/owner后才能一次性双账本转账；不依赖EXEC状态、不造奖励、不可由VM配置或补充。实际A credit可由原资源读取感知并消费，审计a_credited零值省略保持旧序列化，epoch正常刷新、无自动续费。新增kernel正/负契约测试及workspace回归，尚未运行结果；环境验证成本暂未建模，不是市场或自维持生态。后续验收后用真实个体试验；G0 GO/G1,G2 NO-GO，正式E001未校准/预注册。


### 2026-11-24 — 有限资源结算专项2/2验收、全量回归阻塞与自主赚取消费（待验收）
- [有限结算#45407022797](https://github.com/iFrankStudio/OCEE/actions/runs/45407022797) @7b440dc 内核专项**2/2成功**：A25、托管150，真实提交费20、credit100→A105/托管50，随后真实资源读取和消费、重复CAS/覆盖零credit、精确复演、epoch刷新A25而托管50不补充。9类错误/失败提交零credit、默认关闭序列化、未支付托管跨epoch保留通过。全量workspace失败（旧S28诊断match未覆盖已有5个维护Action，E0004；非新枚举），未宣称workspace通过。@89b31bf仅增加诊断格式分支（无行为/判据变化），[回归重跑#45407357391](https://github.com/iFrankStudio/OCEE/actions/runs/45407357391) in_progress/待验收。
- [自主赚取消费协议](./experiments/E001/DEV-S29-AUTONOMOUS-EARN-AND-SPEND.md) @66c8f83，源码/工作流 @4e76a44，[CI #45407569736](https://github.com/iFrankStudio/OCEE/actions/runs/45407569736)。独立内核集成pilot，两个未硬编码答案的单字母ID/输入长度、seed129已知Len先验；相同root/任务对比LOW1A/HIGH1000A，托管1000A，先真实安装后epoch16计划资源变更256A，12tick窗。VM真实读取公开任务、动态ID路径、Len+SHA256+canonical receipt、实际Write获得真实资源，随后尝试2048B真实工作对象。预期两组均正确提交并获相应credit，HIGH可完成额外工作且累计费用>257A，LOW因真实Gas不能创建；严格逐tick费前扣除/转账/托管守恒、真实产物/版本/独立验证及精确复演，无激活后研究者写入。已知算法与后续工作由研究者预设，不是自主发现价值/选择消费或自维持生命；待原生CI，G0 GO/G1,G2 NO-GO。


### 2026-11-24 — 自主赚取消费1/1验收、有限生产链（待验收）
- [自主赚取消费#45407569736](https://github.com/iFrankStudio/OCEE/actions/runs/45407569736) @4e76a44 **1/1原生**，四轨迹。相同root82b103c6…7367B、真实安装469A，初始运行256A，两种任务a/b均真实提交@17。LOW各credit1、费用168/171A、余额89/86A、10次Gas、未完成且无工作对象、托管999A；HIGH各credit1000、费用445/448A（提交后295A）、余额811/808A、0 Gas、实际2048B对象、return1、托管0。全逐tick费用先扣/真实转账/独立产物/精确复演通过，存储7711/9759B。已验证收益资助额外工作，算法和消费仍预设，非自主价值/消费选择。
- [全量回归#45407357391](https://github.com/iFrankStudio/OCEE/actions/runs/45407357391) @89b31bf in_progress/待验收，不得提前视为已通过。
- [有限生产链协议](./experiments/E001/DEV-S29-FINITE-PRODUCTIVE-CHAIN.md) @9d92a1f，源码/工作流 @cc3b27e，[CI #45408841115](https://github.com/iFrankStudio/OCEE/actions/runs/45408841115)。可选sequence扩展2..3唯一ID、总价<=固定托管上限，合法提交后同tick发布下一真实任务、版本递增、私有settled ledger、无续资；旧单任务序列化省略新字段。相同VM动态读取和计算三项任务，正credit反馈才继续；两组初始256A/托管1400A，仅首任务价格1/1000不同，后两项各200，预测LOW仅首项、HIGH三项且后续由赚取A资助。冻结12tick窗及严格费用/真实转账/正确产物/版本/replay；负对照重复ID、超额承诺、错交、不重复领取、跨epoch托管持久。正credit字段为零时省略，当前pilot根未实现通用缺字段处理；仅测试合法已支付路径及真实Gas边界。有限环境需求和预设算法/循环，非持续自维持或自主学习；G0 GO/G1,G2 NO-GO。


### 2026-11-24 — 有限生产链5/5验收、实际成本流动性决策（待验收）
- [有限生产链#45408841115](https://github.com/iFrankStudio/OCEE/actions/runs/45408841115) @cc3b27e **5/5原生**，含原单任务结算2项、原自主赚取消费1项、链负对照1项及生产链1项。相同根792f320d…3500B、真实安装227A、初始256A。LOW只a@17/credit1，读取b@18后9次Gas，实际费用256、最终A1、payer1399、未完成。HIGH实际a/b/c@17/19/21，credit1000/200/200，费用560A，最终1096A、payer0、0 Gas、return3。全部逐tick守恒、正确收据、发布版本、真实转账和精确复演通过。重复ID/超承诺拒绝、错误不推进、旧提交不重复获利、未支付托管跨epoch保留通过。有限给定需求和预设算法，非自主学习或永续生态。
- [全量回归#45407357391](https://github.com/iFrankStudio/OCEE/actions/runs/45407357391) @89b31bf in_progress/待验收。
- [实际成本流动性决策协议](./experiments/E001/DEV-S29-MEASURED-LIQUIDITY.md) @5642b35，源码/工作流 @cbc8d60，[CI #45409819441](https://github.com/iFrankStudio/OCEE/actions/runs/45409819441)。无世界/VM/托管改动。相同结构两根CONTINUE与ESTIMATE，均初始及每次正credit后实际付费读取resource-status，真实前余额+credit-现余额估算上一观察区间全费用，计算当前余额/估算费用；ESTIMATE商0即正常结束、CONTINUE忽略。真实384A初始grant、原a,b,c价格1/1000,200,200、托管1400、12tick窗、seed129。冻结LOW两组仅a，ESTIMATE无Gas正常return1且剩余更多A；HIGH均三项无Gas。真实每tick账本、sensor字段/区间费用、独立正确产物、版本/复演、安装实际计量。公式和决策规则预设，成本由个体真实活动测得，不能保证未来任务可负担或最优，不是自主价值发现。G0 GO/G1,G2 NO-GO。


### 2026-11-24 — 实测成本流动性决策1/1验收与工作负载反例（待验收）
- [实测成本#45409819441](https://github.com/iFrankStudio/OCEE/actions/runs/45409819441) @cbc8d60 **1/1原生**、四条轨迹。CONTINUE/ESTIMATE根5250/5314B，实际安装337/341A。初始grant384A。LOW均a@18/credit1、sensor@16余额376、@19余额164，实测区间213A、商0；CONTINUE读b@20、7次Gas、未正常结束、累计371A、最终14A；ESTIMATE正常return1、0 Gas、累计259A、最终126A，运行省112A（含额外安装差4A则总省108A）。HIGH两根均a/b/c@18/21/24，credit1400，sensor376/1162/1136/1110A，区间214/226/226A，商5/5/4，累计713A、最终1071A、0 Gas、return3。逐tick真实费用/托管转账/资源报告/独立产物/精确复演通过。成本估计非未来保证。
- [工作负载变化反例协议](./experiments/E001/DEV-S29-MEASURED-LIQUIDITY-LIMITS.md) @5092f10，源码/工作流 @b3a47cf，[CI #45411437080](https://github.com/iFrankStudio/OCEE/actions/runs/45411437080)。复用原两个已验收根，固定INCREASE短输入→255B输入，DECREASE128B输入→短输入，a/b/c价格1/200/200，托管1400，seed129、真实grant网格384/512/640/768/896/1024/1280，12tick，28条独立轨迹。预先冻结存在性预测：至少一个INCREASE商>=1却读b后真实Gas无b结算，至少一个DECREASE相同首测量商0、ESTIMATE正常停止而CONTINUE实际获b正确结算。低预算未完成首测量者原样记录，不删样本；全部真实账本、资源报告、独立收据、精确复演，未改变VM/世界/策略。无拟合或新学习能力，待CI；G0 GO/G1,G2 NO-GO。
- [全量回归#45407357391](https://github.com/iFrankStudio/OCEE/actions/runs/45407357391) @89b31bf in_progress/待验收；未声称全量通过。


### 2026-11-24 — 工作负载变化反例验收、规模条件预测（待验收）
- [成本变化#45411437080](https://github.com/iFrankStudio/OCEE/actions/runs/45411437080) @b3a47cf **1/1原生**，28轨迹全部守恒、实际正确收据、资源报告、精确复演。INCREASE512/640A：估计213A，余额292/420A，商1，读b后7次Gas且只a结算；INCREASE768/896A：b实际区间497A，余额251/379A，ESTIMATE停于2，CONTINUE完成3。DECREASE640A：估计349A，余额284A，ESTIMATE正常停于1、0 Gas、余额246；CONTINUE完成3、0 Gas、余额193。DECREASE384A：余额28A、商0，但两根仍8 Gas且余额0，未完成停止决策。冻结存在性预测两项均通过。不能将上一实际成本视为未来保证；决策开销及停止能力本身也是资源约束。
- [规模条件预测协议](./experiments/E001/DEV-S29-SIZE-CONDITIONED-LIQUIDITY.md) @6514653，源码/工作流 @ed7898e，[CI #45412479690](https://github.com/iFrankStudio/OCEE/actions/runs/45412479690)。两根均先真实付费读取下一任务，携带上一实测区间成本C、输入长度L与资源读数A，计算floor(C*(128+N)/(128+L))，LAST选原C，SCALE选规模条件估计。128是研究者平滑先验，非训练参数；使用既有U32运算及9轮有界二进制乘法，不增加内核/VM能力。模型成本可能明显增加；按前次sensor余额比较完整预测区间，非免费当前余额。首项不预测、三项有界，同原INCREASE/DECREASE及7档grant，固定15tick窗内28轨迹。冻结至少一个INCREASE避免无效Gas且正常结束、至少一个DECREASE恢复任务且无Gas，记录所有损失；无调参、保留红。真实付费、正确结算与精确复演，待CI。G0 GO/G1,G2 NO-GO。
- [全量回归#45407357391](https://github.com/iFrankStudio/OCEE/actions/runs/45407357391) @89b31bf in_progress/待验收，不是最新HEAD全量验收。


### 2026-11-24 — 规模条件预测完整假设未通过、低开销加法预测（待验收）
- [规模条件#45412479690](https://github.com/iFrankStudio/OCEE/actions/runs/45412479690) @ed7898e **0/1**，28轨迹全部执行且逐tick账本、真实sensor、正确产物、精确复演检查通过；最后预设存在性断言 recovered=[] 失败，原红保留。两根8459B、实际安装537A。INCREASE768：LAST只a、10 Gas、费用751/最终18A；SCALE只a、正常return1、0 Gas、费用704/最终65A，avoided=[768]。DECREASE640：LAST正常停1/0 Gas/最终57A，SCALE尝试b/10 Gas/最终2A，无恢复产出；INCREASE896：LAST停2/0 Gas/最终48A，SCALE尝试c/7 Gas/最终16A。INCREASE1024 SCALE三项/233A vs LAST两项/176A，但并非预设DECREASE恢复判据。lost=[]仅任务数、不代表无Gas或资源退化。实际短任务后续区间330A vs前版226A，增加104A为付费观察/模型/控制等联合变化，不孤立归因于乘法。长任务区间601A vs前版497A。模型预测成本本身显著改变可负担性；不得称已验收。
- [低开销加法模型协议](./experiments/E001/DEV-S29-ADDITIVE-LIQUIDITY.md) @8ea1b14，源码/工作流 @f436480，[CI #45413439198](https://github.com/iFrankStudio/OCEE/actions/runs/45413439198)。独立继承同一真实付费下一任务读取、上一成本C/长度L/新长度N，删除9轮乘法，用C+N-L（非零L下显式下溢guard、零估计停），两根均同样计算，ADD选修正估计，LAST选C。1A/字节为研究者预设先验，非拟合/学习；不改变世界/VM/托管，固定原两tape、7档grant、price1/200/200、endowment1400、15tick、28条轨迹及两个原存在性判据，原红与所有实际费用、Gas/损失、产物和复演保留。尚未验收，G0 GO/G1,G2 NO-GO。
- [全量回归#45407357391](https://github.com/iFrankStudio/OCEE/actions/runs/45407357391) @89b31bf in_progress/待验收，不代表最新HEAD全量验收。


### 2026-11-24 — 低开销加法预测验收、双样本在线系数选择（待验收）
- [加法预测#45413439198](https://github.com/iFrankStudio/OCEE/actions/runs/45413439198) @f436480 **1/1原生**，28条轨迹，avoided=[640], recovered=[640], lost=[]（任务数）。两根7386B/实际安装470A。INCREASE640 LAST一项/10 Gas/费用639/最终2A，ADD一项/正常return1/0 Gas/费用613/最终28A；INCREASE896/1024 ADD三项/0 Gas/最终287/415A，LAST两项/0 Gas/最终230/358A。DECREASE640 首段352A、余额279A，LAST正常停一项/148A，ADD实际三项/166A，均0 Gas；后续短区间239A（此前比例330A、未检查下一任务226A），长区间510A。全部真实账本、sensor、正确收据及精确复演通过。INCREASE384/512及DECREASE384仍有Gas，非普适安全/收益，固定1A/字节非学习。
- [双样本在线系数选择](./experiments/E001/DEV-S29-TWO-SAMPLE-COST-FIT.md) @fdd2502，源码/工作流 @fe9d000，[CI #45415180427](https://github.com/iFrankStudio/OCEE/actions/runs/45415180427)。两根FIXED/FIT从已验收ADD派生，均携带前次真实成本/长度和候选系数；初始1，第二次实测后若成本增量>=长度增量+其一半（向下取整）则选2，否则1。仅测试训练长度增加的fixture，第三项真实付费读取后FIXED用1/FIT用所选系数，双方实际计算同类候选长度、带下溢guard。固定密集字节255/稀疏字节7的128B与255B后续任务、首3B、9档grant768..1792、seed129、price1/200/200、endowment1400、15tick、36轨迹。冻结主验收：第三任务读取时实际VM寄存器96与独立真实前两区间拟合结果一致、两根测量/所选系数相同、跨tape至少观察系数1及2，报告实际结果及开销，无预设方向性收益或调参。候选类和拟合规则仍由研究者预设，仅单episode在线参数选择，未实现持久/开放式模型学习；G0 GO/G1,G2 NO-GO。
- [全量回归#45407357391](https://github.com/iFrankStudio/OCEE/actions/runs/45407357391) @89b31bf **cancelled**，专项结算通过、全量步骤被取消，仍不能视为全量兼容性验收；未声称最新HEAD全量通过。
