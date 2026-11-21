# S45 — 配对协议种子消融：一项尚未执行的开发集实验

**状态：已冻结候选实验设计，等待原生GitHub Actions验收；不得以测试通过推断所有配置成功。**

## 问题与预先冻结的比较

既有 S45 2/6/8 步延迟成功证据来自同一世界 seed55 / P2 protocol seed42。高延迟下的行为也可能取决于协议实例的私有 opcode、字段排列和标记，而非仅等待机制。因此选择两组**新增的开发集协议种子** 17 和 93，固定世界 seed55、活动任务、许可、资源预算与全程消息延迟8步；对每个协议实例做旧 `S44`（全局共享128等待）和 `S45`（Phase2每候选6、总896）**两臂配对运行**。

四次运行参数由测试代码写死、提交前预先冻结；不得看到结果后增删困难种子或改成功标准。两臂均采用自己原本的、真实VM生成并内容寻址的P2 root；无一臂预读生成器私有 opcode 或 world B gas。P2 protocol seed仅作为研究者的 **特权世界配置**，不是个体输入。两臂程序并不完全相同：差异限定在已提交S45替代S44的活动等待政策，其他研究者先验相同。

## 可评价量与安全硬断言

每次打印机器可检索行 `S45_PAIRED_ABLATION`：seed、arm、delay、真实终止返回、是否成功、B扣费、真实REMOTE_EVAL SEND的u64 ID列表、成功的 `ConfirmResponseFor` ID、共享与局部等待计数、总tick、初始活动P2程序hash及重启记录。日志中所有四行和所有失败必须报告；不能只展示成功场景。

无论哪臂、哪协议：**B不得超过24，真实B任务SEND不得超过一次，不应出现无任务漂移的root重启，两个等待计数不得越上限；世界可经snapshot重放。** 成功条件严格要求 `Value::U32(1)`、B24、真正REMOTE_EVAL及其ACK认证，并且完成结果等于真实远程工作项操作数fold32；失败不得留下成功RESULT对象。若任一硬约束失败，原生CI必须红。

该测试的通过**只代表安全断言**。活性结果必须从上述四行 `success=true/false` 独立汇总；同一固定环境paired比较，不等于正式泛化统计、H1/H2/H3验证或预注册样本。本试验仍是研究者预置 P2 TLV 解析、112 opcode候选、固定专用工具和明示RESULT扫描的开发程序。若多个种子失败，先检查消息轨迹、算法预算与协议生成器，并原样保留阴性结果，不得把S45宣传为跨协议稳健。


## 2026-11-21 — 配对实验真实结果：CI安全成功，但活性四臂全部失败

[原生GitHub Actions #44916855269](https://github.com/iFrankStudio/OCEE/actions/runs/44916855269) @ **`519f9ccb20d389400490eb6accee61b82397d1b9`** completed/success，单项Rust实验包含4条完整真实活动P2轨迹（129.46s），同时满足事前设置的安全/快照断言。

| 协议seed | 旧S44 | 原S45（仅Phase2 scoped） |
|---|---|---|
| 17 | `U32(0)` / B0 / 无真实RemoteEval | `U32(0)` / B0 / 无真实RemoteEval |
| 93 | `U32(0)` / B0 / 无真实RemoteEval | `U32(0)` / B0 / 无真实RemoteEval |

原生日志逐行记录的两个程序哈希分别是S44 `bb1a2ad1c790acdc9686b091a61e4b4bf84d39c9fbd6b6c994adebeb8f0e5fdf` 与S45 `b400828fdd204fe2d1e7b713145296cf13796e6ef159c02325640b609726ea3b`；四组均为 `s23_wait=128`、`s45_wait=0`、`step=411`，无虚假B执行、无根重启。**绿色代表安全断言通过，不代表P2协议适应成功；活性成功为0/4。**

这揭示原S45只将Phase2 RESULT按候选opcode独立等待，而更早的Phase0 REMOTE_EVAL opcode扫描仍沿用全程序共享128次等待；在这两组新协议参数下，Phase0先耗尽旧额度，个体尚未Send真实REMOTE_EVAL就fail-closed。这个阶段定位由phase0期间未生成任何B消息和loop_counters的具体键强支持，仍需新增代码对照才能判断局部等待是否为全部根因。

**在看到阴性结果后设计的修复必须明确为新开发研究，不能回填修改本次已冻结的原生样本。** 下一测试保留完全相同协议seed17/93、全程delay8和任务配置，仅为S45活动VM增加受限的Phase0等待，每个opcode 6次、总896次；旧S44图不变，不能新建B重试权限。只有真B24、恰好一次真实REMOTE_EVAL、经服务确认的ACK及RESULT、正确fold32与快照一致才能验收。即使通过，也不等于正式E001跨协议泛化。


## 2026-11-21 — 同批协议seed17/93的Phase0 scoped修复：原生2/2真B成功

注意严格区分**两个不同的已提交源码快照**：
- **原版对照**：[Run #44916855269](https://github.com/iFrankStudio/OCEE/actions/runs/44916855269) @ `519f9ccb20d389400490eb6accee61b82397d1b9`，4条安全断言通过、4条真实任务失败。S44和原S45两臂均Phase0提前共享s23=128耗尽、B0。此阴性结果不可被修改或回填。
- **失败后提出的工程修复**：[Run #44917248521](https://github.com/iFrankStudio/OCEE/actions/runs/44917248521) @ `4cbf8743b63eff8de55ae1d8bdd3b872f9e11d8e`，原生Rust用例 **1 passed / 0 failed；407.03s**，内部两条S45新轨迹均达到 `Some(U32(1))`，真正B24且只一次，真REMOTE_EVAL SEND仅一次，S43B `ConfirmResponseFor` 认证ACK和RESULT、计算结果与独立 `fold32` 一致，世界快照哈希一致且没有不当Root重启。

| 协议seed | 原S45（原生阴性） | 修复后S45（原生正例） | Phase0 scoped waits | Phase2 scoped waits | 完成tick | 真实B消息u64 ID |
|---|---|---|---:|---:|---:|---:|
| 17 | U32(0) / B0 / 411 tick | U32(1) / B24 | 354 | 312 | 2140 | 120 |
| 93 | U32(0) / B0 / 411 tick | U32(1) / B24 | 318 | 360 | 2198 | 108 |

原版S44在两协议实例均失败的事实没有改变。Phase0修复针对的新观察是：先前没有进入Phase2，原因是旧S23共享128等待在前置REMOTE_EVAL opcode扫描耗尽；旧S45只为Phase2处理了同一症状。新图在`s41_check_drop`的非Phase1入口按phase分别路由Phase0/Phase2的`RepeatScoped`，二者分别有**每opcode 6次、每阶段总896次**，仍保留Phase1真实ID与一次安全恢复、Phase3旧边界、不新建工具和重发权限。

新代码改动后两固定seed的校准属于**使用原失败样本进行修复**（development-set regression），**不是**见过结果后仍可称作“盲测成功”的独立证据；它只强力支持“此两例Phase0等待额度是可修复的关键故障因素”。原实验的成功率仍是S44 0/2、旧S45 0/2，不变；修复后S45本次仅2/2。不能直接宣称未知协议泛化、E001 H1/H2/H3、现实网络exactly-once或赛博生命产生。正式 E001 G0 GO、G1/G2 NO-GO及未校准/未预注册不变。

两个本轮临时工作流push触发均已恢复仅手动`workflow_dispatch`，防止后续无意重复执行约7分钟的原生全程多seed探索。
