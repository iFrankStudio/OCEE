# DEV — S29 真实反馈下的错误继承工具局部修复与 CAS 安全采用（运行前冻结）

**状态：运行前冻结协议，已补录真实原生4/4和两次红例。** 基线 `ad4cc0a43d24610cd9b4fe75da805f58fc9014b6`。参考S29原生 [#44673328957](https://github.com/iFrankStudio/OCEE/actions/runs/44673328957)：正确跨家族复用与两例+held-out均已验证；但在目标 `P2 world_seed=129` 的 Len 任务导入来源 `P1 world_seed=43` 的 XorFold 工具时，根真实Exec后 U32(0) 提前结束、B0、无预测工件。上述单次失败**已被看过**，不是独立盲测。

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

## 2026-11-23 — 真正原生4/4：错误继承工具反馈修复、一次性复核、CAS安全采用

最终 [GitHub Actions #45107700394](https://github.com/iFrankStudio/OCEE/actions/runs/45107700394) @ `e7f602399be6f6039dd07ccbb022cf26e8dccd45` **completed/success，FRESH/MATCHED/WRONG/STALE四个独立Rust jobs均1 passed/0 failed**。活动新S29根四条件同一内容hash `f8e5d42b1d4286395b6b3453801264c819e4b97d53159d8c24cb04e480b4b5fe`；目标皆P2 world seed129/protocol seed42，12,000 A gas每epoch、16,384 tick上限、真实S29 P1→P2继承、原VM强制静态无界环禁令和所有世界权限成本不变。

**算法与信任边界**：源程序来自实际P1 seed42/43各自自主运行并真正通过两公开样例、真实heldout和B20的 `Len`/`XorFold` 可执行版本，非宿主伪造输出。研究者在目标运行前正常付费导入来源工具和注册表，目标活动根只能读自身已注册hash、执行已安装纯VM、读取世界公开双标签和计算隐藏答案未知的第三输入。根首个样例真实Exec和标签不符时（旧XorFold在P2 seed129的`[32,3,8]`实际返回43，公开正确标签3），通过已有3候选 `Len/XorFold/ByteAt0` 比较公开两标签，真正调用 `RewriteOp` 生成新的内容寻址程序，按世界费用Write安装，然后CAS `expected_version=0` 替换旧注册表；成功后使用**另外一条仅通过一次**的修复后校验路径，真正Exec两个已标注例子及不同heldout例子，才允许进入既有B协议后续计算。不存在世界私有答案传入活动VM；校验选择、重写、采用和继续运行均可在审计中追踪。

| 完整原生账本 | FRESH | MATCHED | WRONG | STALE |
|---|---:|---:|---:|---:|
| 导入的实际P1工具 | 无 | Len | XorFold | XorFold |
| 首次旧工具Exec样例 | 无 | 实际Len三次（见下一行） | **43，错误** | **43，错误** |
| 新Len在目标运行期真正Exec次数 | 3 | **0次重新生成；3次复用来源Len** | **3次新版本Exec** | 0次（未采用） |
| 目标运行中新Len版本成功Write次数 | 1 | **0** | 1 | **1，孤儿版本** |
| 注册表CAS0实际反馈 | Conflict | 不调用 | **Ok** | **Conflict** |
| 不存在注册表时的None写入 | Ok（新建） | 无（研究者先导入） | 无（研究者先导入） | **Conflict** |
| 最终Registry版本 | 0（新Len） | 0（继承Len） | **1（更新为Len）** | **1（旧XorFold保持）** |
| heldout预测与独立真值 | 5 / 5 | 5 / 5 | **5 / 5** | 无 |
| 原生P2落盘结果与独立真值 | 1156832460 / 1156832460 | 同 | 同 | 无 |
| 真实B gas | **20** | **20** | **20** | **0** |
| VM A gas | 17149 | 16979 | 17197 | 398 |
| 世界Action A gas（含安装） | 6835 | 6837 | 6867 | 6255 |
| **总A gas** | **23984** | **23816** | **24064** | **6653** |
| 结束world step | 1183 | 1183 | **2051** | 14 |
| 根最后返回 | U32(1) | U32(1) | **U32(1)** | **U32(0)** |
| 自主Denied/InvalidCode / 未授权重启 / 快照恢复 | 0/无/一致 | 0/无/一致 | 0/无/一致 | 0/无/一致 |

同一个目标P2下，WRONG相对MATCHED总A多248 gas并多868 steps，但两臂预置工具字节、来源和初始世界状态不同，**不构成等起点因果净收益估计**。MATCHED的0新版本Write要求从目标个体开始自主运行**之后**计数，不能把研究者付费导入的那次版本Write误计成个体重新生成。STALE的两次Conflict分别为活动根试图CAS0修改已有version1及尝试None新建已有对象；只剩已完成付费Write的新Len不可变版本对象但没有注册采用，是真实“孤儿版本/不可逆创建”限制，不能称为原子回滚，也不能把安全拒绝说成完整恢复。FRESH在确实没有注册表时先尝试CAS0返回Conflict，再仅一次None Write创建成功；这是有限预置容错而非动态版本探测通用协议。

### 原生红绿链必须保留

- **首次[#45107240682](https://github.com/iFrankStudio/OCEE/actions/runs/45107240682) @c878b64**：native四臂 **0/4**，全部在活动根静态校验阶段 `InvalidProgram("unbounded control-flow cycle")`，没有世界任务语义结果。最初把“第一次失败后重选”跳回原校验块形成隐式无界循环。
- **修正 `cd4d6b0` 后 [#45107371940](https://github.com/iFrankStudio/OCEE/actions/runs/45107371940) @9e91a00**：修复后校验只执行一次、完全单向；STALE/FRESH/WRONG三组真世界协议都达成各自冻结断言，**3/4**。MATCHED实为正常正确复用、成功B20，但测试误把运行前研究者导入工具的合法Version Write计成新生成，导致测试断言红色；不能当4/4。
- **修正 `07e8472` 后 [#45107700394](https://github.com/iFrankStudio/OCEE/actions/runs/45107700394) @e7f6023**：只将版本审计限制在目标根真正开始自主运行之后；不改任务、资源、初始工具、VM、实验条件或活动控制图。最终四臂4/4，静态无界环限制始终保留。

**科学判定**：这是原生VM在公开标签下从“已有工具语义不符即死亡式停止”提升到“先证伪继承工具、有限候选选择、修改合法可执行代码、付费采用、正确调用并完成原生P2计算”的真实开发级适应演示。两标签、三原语搜索空间、控制图及修复契约仍由研究者预设，没发现新的原语、没有跨开放空间演化或正式30-seed A/B/C/D；当前 G0 GO、G1/G2 NO-GO，正式E001未校准或预注册。下一可证伪缺口是**第二标签才冲突的修复路径**、可信且能回滚的工具升级、跨episode对照与更多独立heldout。旧完整checker原生 [#45103750605](https://github.com/iFrankStudio/OCEE/actions/runs/45103750605) 现在明确`completed/cancelled`，原VALID job cancelled，旧3/4不可追认为4/4；后来独立的锁定checker研究 [#45105207973](https://github.com/iFrankStudio/OCEE/actions/runs/45105207973) 为另一根四臂4/4。此定向S29 workflow验收后改为仅手动。
