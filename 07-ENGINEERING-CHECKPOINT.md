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
