# DEV — S29 真实反馈下的错误继承工具局部修复与 CAS 安全采用（运行前冻结）

**状态：运行前冻结，未取得本项原生证据。** 基线 `ad4cc0a43d24610cd9b4fe75da805f58fc9014b6`。参考S29原生 [#44673328957](https://github.com/iFrankStudio/OCEE/actions/runs/44673328957)：正确跨家族复用与两例+held-out均已验证；但在目标 `P2 world_seed=129` 的 Len 任务导入来源 `P1 world_seed=43` 的 XorFold 工具时，根真实Exec后 U32(0) 提前结束、B0、无预测工件。上述单次失败**已被看过**，不是独立盲测。

## 核心假说及限定

当前S29有研究者预置的3个合法通用变换原语（Len、XorFold、ByteAt0）和两个公开标注样例以及独立隐藏答案的held-out输入。研究者不对 VM 提供当前选中的操作或隐藏答案。本项仅修复一个明确的缺失：已有注册工具**第一次真实付费EXEC** 与公开样例1不符时，不再直接失败，而是保留已观察到的同两个真实标签，在活动VM里重新运行原 S29 的三个原语候选比较、真实 `RewriteOp` 生成另一个程序、付费 `WRITE` 到不可变版本库，尝试 `expected_version=Some(0)` 的原注册对象 CAS0；成功则真的Exec新程序在两公开样例和held-out输入，并继续既有真实P2远程工作项。

安全：所有初次创建的注册表此时 absent，因此版本0 CAS必得到Conflict；**仅对此可见Conflict**允许再付费一次 `expected_version=None` 创建注册表；若是已有版本1的陈旧注册表，则CAS0失败且None也失败，安全停止、不使用错误工具、不尝试远程B、不写成功预测。此处 CAS0+None 是由研究者提供的确定性有界重试策略，不推断为系统提供了一般可验证的“注册对象不存在”证明或并发无竞态。所有错误都禁止无限重试。第二个样例或 heldout 后续失败仍按旧根fail-closed，本项只研究**继承工具在第一公开样例明确冲突**后能否自主重选和原子CAS采用；不能把新产生但未登记的孤儿版本称为完整回滚。

冻结研究条件：S29 P2 `Family::P2`、协议seed42、目标 `world_seed129`、默认A12000/epoch、单次运行至多16384 tick、世界原生P1/P2已预装共同协议专家的固定hash、统一活动根hash。研究者只预先安装或注册 donor，全部有偿，不替活动VM挑选修复算法。明确列明不是真正新算法发明/跨未见语法学习，只是已有三原语有限空间里针对公开标签的受费反馈恢复。

## 独立原生四臂（冻结）

| arm | 研究者前置授权工具 | 注册表初态 | 活动根期望 |
|---|---|---|---|
| FRESH | 未导入S29工具 | absent | VM选Len，创建版本与新注册表（经历CAS0 Conflict后有界None创建），真实3次Exec、正确heldout、完成B |
| MATCHED | 源P1 seed42真实Len工具 | version0与hash存在 | VM复用，两个公开标签Exec通过，无新版本产生，正确heldout、B成功 |
| WRONG | 源P1 seed43真实XorFold工具 | version0与hash存在 | 首次真实Exec标签失配，活动VM从现有公开标签重选Len、生成新程序，通过真实CAS0替换旧注册哈希；3次新的Exec、正确heldout与B成功 |
| STALE | 同一真实XorFold donor | **通过一次合法研究者CAS0将注册对象提升至version1（字节保持相同）** | 首样例失配后活动VM实际选择与生成Len，但CAS0冲突、None重试冲突；禁止覆盖version1、禁止持久化预测、B0、根失败。不把新产生的不可变版本视作完成修复 |

测试必须审计候选 `Write(/self/versions/<hash>)` 内容hash、旧工具和新工具每次实际 `Action::Exec` 输入+值+收费、注册表前后版本和字节、`Action::Write(REGISTRY, CAS0)`真实Ok/Conflict、None重试、世界B账本和来自C的真实RemoteEval/最终结果、P2新heldout独立 `hidden_transform`（仅宿主端可访问）、最终return、Gas/VM/action费用、step、快照恢复一致及无Denied/InvalidCode。失败臂必须是预设CAS冲突所致，不允许预算Gas造成的假失败。

MATCHED / WRONG / STALE具备相同root和环境，但初始工具/注册版本不同；FRESH没有导入工具，绝不以跨臂gas差作为同起点因果效益。特别强调CAS0若遇到了真实并发更改后的version1，仍需保证注册对象不被本root覆盖。每种情况最多一次原始候选检验+一次三原语重新生成，不无限循环。

该实验仅提供**研究者定义的有限修复策略在活动VM里的真实执行**，不能升级G1/G2，也不替代正式E001所要求30seed A/B/C/D配对、结构性外推和长期生命周期。首次原生红灯永久保留；以真实完成状态为准，不因CI绿色就推断任何超出断言的能力。CI仅运行定向四组并在验收后改为workflow_dispatch；旧 [#45103750605](https://github.com/iFrankStudio/OCEE/actions/runs/45103750605) 已最终 `completed/cancelled`（旧VALID cancelled），不能当作4/4成功。
