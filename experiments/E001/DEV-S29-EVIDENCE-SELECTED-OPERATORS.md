# S29 — 从公开标注样例选择通用算法并跨协议继承（开发实验）

**S29 开发协议草案，2026-11-20；未正式预注册 E001，不升级 G1/G2。**

## 相对于 S28 的变化

S28 由研究者直接提供 `Len(Bytes)` 作为唯一正确模板和固定长度12的通关条件。S29 **不在活动代码中提供当前世界选哪一种算法的答案**：它复用世界内核已存在的 `/public/E001/transform`，其中有两个已标注输出的输入样例和一个不公开答案的 held-out 输入。隐藏映射由 world seed 定义，`hidden_transform` 只能被世界装置和完成后的研究端使用，活动 VM 不能调用它。

**研究者依然明确提供**：有界的三原语搜索空间 `Len(Bytes)`、`XorFold(Bytes)`、`ByteAt(Bytes,0)`，按长度→异或→首字节的检验顺序、已知图模板、`RewriteOp` 修改槽位、正误比较标准、持久化及任务编排。个体只是从有限既有候选中依据公开例子选择，并不是自主发明任意新算法，也不会产生语言级 CALL 或开放式演化。

## 个体活动流程（原生 VM，不是测试宿主决策）

1. 公共 root 读取 `/public/E001/transform`，解析两组 `input/output` 与 held-out input。公开示例是真实可见的观察，不含隐藏世界种子或隐藏答案。
2. 活动 root 先通过真实 `WorldCall::Read` 查询既有注册对象 `/self/tools/portable-transform-s29`。若存在，只把它当成候选而不是真理：**需要两次真实 `EXEC` 均与目标世界公开样例相符**。
3. 若不存在，活动 VM 对两个样例计算三种候选输出并选择两例都吻合的第一个算子；通过 `RewriteOp` 写入原本 `ConstU32(0)` 占位的类型化指令，按 `OCEE-PROGRAM-V1\0` 内容规则计算 SHA256，受费 WRITE 不可变程序版本与 registry。
4. 即使已生成程序，仍经两次隔离 `EXEC` 验证两个公开示例；再针对 **held-out 输入真实 `EXEC`**，把预测值持久保存到 `/self/results/portable-transform-prediction`。保留输入真实答案不出现在活动程序中，研究端只能在运行结束后调用隐藏 oracle 核验预测。
5. 完成上述能力验证后进入既有 S28 根程序通用长度门控、公开 HELLO P1/P2 专用程序选择和真实 C→B FOLD32。**S28 的固定长度门控仍是研究者先验，不得将整套远端工作能力归因于 S29 新选择的算子。**

## 具体对照和严格归因

- 冻结 root hash 与研究者预装的 P1/P2 专家库在全部世界相同。P1 来源世界 seed42/43/44，对应公开例子的三种隐藏算子；P2 目标世界 seed129/130/131（相同 seed%3 映射但数据内容不同），protocol seed 均42，以固定远端协议实例降低无关混杂。分别查看选择、生成、三次 EXEC 和已保存的 held-out 正确率；不是统计有效的 3-seed 确认性样本。
- 目标 P2 的可见导入组通过两次受费 WRITE 继承来源 P1 真实生成的程序和 registry。断言 P2 只通过本地样例验证并执行继承程序，没有重新生成该程序，held-out 预测独立正确，B gas20，FOLD32 独立正确。
- 等成本不可见工具 sham：同一程序字节、同等写入次数和费用，只把 registry key 改一字符；目标必须重新进行候选选择。因为内容寻址相同，第二次版本 WRITE 可能返回 `Conflict`，这是正常不可变版本行为，必须审计而非视为继承证据。比较可见/不可见的终态 A gas 和 ticks，不用冷启动代替节奏相同的假对照。
- 错误语义继承：把来源 `XorFold` 程序导入期望 `Len` 的 P2 世界。虽然可执行，不能通过该目标的公开样例；活动 VM 必须在 B 计算之前停止并保留错误轨迹，禁止只凭 registry/hash 就自动信任。
- 验证按来源和目标隔离，未复制旧 continuation、工具学习后的 B 结果、世界权限或已完成回执。逐次 `EXEC` 都从内核事件审计查收；目标独立预测由世界专属隐藏规则核验。

## 成功、失败与下一研究门槛

本协议的初步成功标准是：**三种已给定通用算子都被活动 VM 依据两组公开例子正确选择，并在跨家族环境中真实 EXEC 且对不同的 held-out 输入预测正确**，继承组相对等成本不可见 sham 不增加成本并有明确可追溯来源；错误语义程序不会被盲目信任。若种子/协议版本出现失败应按真实日志留存，不为通过筛选 seed。

局限：即使全过，本实验也只证明**有限候选中的反馈驱动选择+可执行工具跨家族迁移**；未检验三原语之外的新功能发现、组合型算法的生成、自动变更验证目标、未知类型语言/服务的一般适应性，也没有开放式演化。保持 G0 GO，G1/G2 NO-GO。

[核心 Rust 实现](../../crates/organism-seeds/src/s29_public_transform.rs) · [原生因果测试](../../crates/organism-seeds/tests/s29_public_transform.rs) · [专用 CI](../../.github/workflows/s29-public-transform.yml)。


## 2026-11-20 原生实验验收

- 首次 [S29 CI #44673179538](https://github.com/iFrankStudio/OCEE/actions/runs/44673179538) **失败于 Rust 测试编译**：测试保存不同源 donor 的两个独立 if 重复 move 非 Copy 的 hash/program。通过 [4be5024](https://github.com/iFrankStudio/OCEE/commit/4be5024a2dd2a89595b0aada08a6a9d078cb0043) 改为互斥 else-if 后未更改科学判据或世界 seed。失败记录保留。
- [修订后的 **S29 定向原生 CI #44673328957**](https://github.com/iFrankStudio/OCEE/actions/runs/44673328957) 已直接核验 **completed/success**，原生 Rust **1 passed、0 failed，用时276.42秒**。初始冻结 root hash 的相同性、候选内容 hash 三种不同、两例 EXEC + held-out EXEC 三次成功、P1 来源 VM 的版本写入、P2 目标消费已导入版本而不自行再次生成，以及状态快照重载均由原生测试断言。
- 配对 `P1_source_world_seed→P2_target_world_seed`（protocol seed 固定42）：`42→129` 自动选择 `Len(0)`，P2 **A 23,626 gas / 1,183 tick，导入25 A gas**，程序 hash `7a48ca2633bc9429c65eab3c0e9c459be956f88e4fb2176e510a9e5fb29a1496`；`43→130` 自动选择 `XorFold(0)`，P2 **A 23,633 / 1,183 tick，导入26**，hash `f0b263f8537295bdaff2afb92bda6527ab91324a3ec26f45f650d6fe70569837`；`44→131` 自动选择 `ByteAt(0,1)`（研究者模板的 id1 恒为0），P2 **A 23,630 / 1,183 tick，导入26**，hash `91168ca0cb6a15ae4bcc19a8fe104313e9c3aabad7e261658628400566f5d757`。三组的 held-out 字节序列与各自 P1 来源不同，三组预测均经研究端私有 `hidden_transform` 比对正确，真实 C→B FOLD32 的 B gas 均为20且结果独立正确。
- **同源等成本 sham**，P2 world129：带可见的 P1 length tool D **A 23,626**，同程序导入却不可见的 sham **A 23,780**，两者导入 **25 A gas**、世界终止 tick 都 **1,183**，因此 D−sham **−154 A gas（约−0.648%）**。sham 确实自主重新选择相同算子，发生已存在版本的内容哈希冲突；不能把 sham 的成功 EXEC(hash) 误记为来自 donor 的真实继承。
- **异语义消融**：将来源 `XorFold` 程序作为合法可执行工具导入目标 `Len` 的 P2 world129，活动 VM 先真实 EXEC，因返回值与目标世界公开标签不符而停止，最终 **A 6,233 / 10 ticks、B gas0、无 held-out 预测**。它证明当前程序会拒绝错误的继承工具，但不会自动丢弃工具、重新选择合适程序；错误语义下的恢复仍有缺口。
- **严格因果范围**：S29 用到世界内核已有的公开双样例，不是新增后门；选择由活动 VM 的普通原语和 Branch 实现。只是研究者预给三原语、算法搜索顺序、图模板和固定目标校验；所谓 held-out 是**单个新的输入**，不是长时间变化的开放环境。三种 seed 各1组，不能推断统计显著性、开放式演化或正式 E001 的 H3。保持 **G0 GO，G1/G2 NO-GO**。S29 与 S28 代码的全 workspace CI 须与定向工作流区分，不能用取消的运行充当完成。
