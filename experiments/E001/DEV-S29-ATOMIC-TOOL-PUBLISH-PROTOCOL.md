# DEV — 可计费、内容寻址、原子工具版本发布与注册 CAS（运行前冻结）

**状态：执行前冻结协议，现已追加首次原生四臂4/4验收（冻结条件不变）。** 基线 `f44f1023c555d695d2e4c22ba266f2fd9285d9db`；延续 [S29首/次公开样例反馈恢复](./DEV-S29-SECOND-LABEL-REPAIR-PROTOCOL.md) 原生[#45115636978](https://github.com/iFrankStudio/OCEE/actions/runs/45115636978) 首次4/4：现有新程序先付费写 `/self/versions/<hash>`，再写 `/self/tools/... ` registry CAS，STALE case在失败时留下新版本“孤儿”。此研究解决的是**有限范围的原子提交/存储一致性缺口**，不是新增自由程序权限、任意回滚，亦非自主发明算法。

## 1. 新动作精确授权边界

新增通用 `WorldCall::PublishTool(registry_key:Bytes, canonical_program:Bytes, expected_version:U32)`，按普通 VM 的 `Yield` 到内核 `Action::PublishTool {actor,registry_key:String,program_bytes:Vec<u8>,expected_version:Option<u64>}`，其中 `u32::MAX` 表示注册表尚不存在；VM仍严格类型和世界动作审计、完整消耗处理指令gas。旧 `WorldCall::Write` 的任何权限/费用/结果语义**不变**。隔离纯VM内调用PublishTool必须因 `IsolatedSideEffect` 被拒绝。

仅允许 `registry_key` 以 `/self/tools/` 开头；不能操作 `/self/active`、`/public/` 或任意其它对象。程序必须完全符合既有 Program 规范：从 `serde_json` 解码、`canonical_bytes()==program_bytes`、`Program.content_hash()` 在原VM MAX_BLOCKS、MAX_PROGRAM_BYTES、无界循环等所有规则下验证成功；内容hash由世界内核对提供的规范化程序计算，**不是宿主根据任务选择的答案**，哈希存储路径固定 `/self/versions/<hash>`，registry内容必须是64字节小写ASCII真实hash。若哈希版本已存在，只允许同一actor拥有且内容字节完全匹配的不可变程序被引用，不能覆盖、注入或冒用他人版本。新版本和注册表一次world action提交，旧注册表只能在 `owner==actor` 且CAS期望版本完全匹配时覆盖。None CAS仅能在注册表不存在时创建v0，Some(v)只能覆盖现存同版本对象创建v+1。

**关键提交前置次序**：验证registry路径/ACL→旧注册表owner和 `expected_version` 一致→验证规范program及内容hash→检查同hash版本对象是否存在和其owner/字节→计算合并存储变化、容量上限和本次完整gas→一次性 `spend_a_gas` 成功→在不可中断世界tick内部**同时写入**版本与注册表、更新`storage_used`。前一步任一失败禁止产生/覆写版本/注册表，也禁止改变`storage_used`；普通调度tick/VM已消耗的gas与事件可以继续发生，但不能称为“整个世界tick毫无副作用”。

**资源记账冻结**：新版本的A费用为 `2+ceil(len(program_bytes)/16)`，注册表更新为 `2+ceil(64/16)=6`，两者单个 `Action::PublishTool` 总收费；若相同不可变版本在此actor拥有的存储里已存在，则只收取注册表更新6 A（不重复存储/写入程序版本）。整体存储变化=`new_program_bytes_if_absent + 64 - old_registry_bytes_len`，严格容量检查，使用checked整数。CAS失败/InvalidCode/Denied/Quota/Gas不允许**部分commit**；成功反馈类型 `Code::Ok`，payload为真实64字节内容hash（供活动VM核验），`gas_charged`为上述完整费用。

## 2. 原生活动个体流程

新S29根应在原真实环境看到首或次公开例子的`Exec`失败后，继续采用预定三候选`Len`/`XorFold`/`ByteAt0`和`RewriteOp`产生程序字节，由VM直接`PublishTool(REGISTRY,program,Some(0))`替换旧工具。若registry确实不存在，第一次CAS0真实Conflict，VM最多一次 `PublishTool(...,None)` 创建；若旧version1，CAS0 Conflict且None Conflict，必须U32(0)而非进入后续工作，**不出现孤儿版本**。成功时还需付费真实`Exec`修复后两例和heldout，继续真正 P2 B计算。保留原S29完整协议任务、原内核VM界、world seed129/P2 seed42/A12000 per epoch/tick<=16384，不把答案交给VM。

## 3. 冻结四臂

1. **REPAIR**：研究者安装合法首例碰撞次例错误 ByteAt(Bytes,1)工具的注册版本v0，活动根真Exec旧工具(3,2)，重新Read公共例子，原子发布真实Len到registry v1；旧版仍在历史不可变存储（这是合法版本**不是孤儿**），新程序只安装一次，Len三次Exec、heldout正确5，真实B20及结果正确。
2. **STALE**：同个不匹配的旧工具，但活动前研究者将注册提升v1。真旧Exec(3,2)→修复候选→`PublishTool CAS0` `Conflict`→fallback None仍 `Conflict`；旧registry v1和存储容量/版本对象集合与活动启动前**完全一致**，新Len hash路径不存在，B0，无成功预测。
3. **FRESH**：registry不存在；活动VM选Len后初次CAS0 Conflict，fallback None Ok；**只创建一次**版本与registry v0，无中间孤儿，不需要旧程序；heldout5、B20及远端结果正确。
4. **FAILURES**：世界原生`Action::PublishTool`独立负对照，不使用任务私有oracle：错误路径Denied、非规范Program InvalidCode、刻意小A额度Gas、刻意小容量Quota、已存在注册表CAS Conflict、现存同hash不同内容/owner拒绝；各case分别核验注册表/程序对象及`storage_used`没有发生局部修改，失败返回真实代码。与三组完整活动轨迹独立，必须明确此臂只检验世界动作原子性而非个体策略。

对于后验代码问题，首次原生红例必须完整保存；仅定向测试此实验，不重跑全workspace或历史study。记录实际各臂root hash、真实Exec、CAS/PublishTool反馈次数、存储对象前后差异、`A_VM`+`A_action`+genesis、真实B/C、world step、snapshot一致和无非法根重启。成功原子性不代表“任意更新可撤销”或“整个world tick事务”；旧程序仍合法持久保存，垃圾回收是另一个受权限与可达性保护的问题。不同研究者导入初态不做等起点效益因果推断。

E001仍是开发集，搜索空间和CAS策略由研究者预设，非自发发明/开放式演化。科学闸门 **G0 GO、G1/G2 NO-GO**，正式E001未校准/预注册。

## 2026-11-23 — 首次原生4/4通过：原子发布消除失败修复的孤立版本

[首次原生 GitHub Actions Run #45119834399](https://github.com/iFrankStudio/OCEE/actions/runs/45119834399) @ `51da269d945ea6391cf729b3bf3c671e58a0cf1b`：**workflow completed/success，四个独立Rust job全部 completed/success、各 1 passed / 0 failed**。本次没有原生红例，也没有提高任何原定世界资源预算或宽松化VM静态验证；通过后专用工作流改为仅`workflow_dispatch`，避免每次push重跑历史4组。原协议于基线`a3a74ef13a01d657f24d8918e9b0037e4b21f25c`先冻结，再实现内核/VM bridge、S29活动根和原生测试。

**实际动作链**：新增普通 `WorldCall::PublishTool` 和 `Action::PublishTool`，只能接收活动VM给出的 `registry_key, canonical_program_bytes, expected_version`，内核按研究者设计的通用机制计算并验证Program内容hash；不能在这个动作中设置任意内容hash或绕过`/self/tools/`授权，也不能用它切换`/self/active`。原版本存在时必须可确认是该actor拥有且字节与内容规范一致；内核检查权限、注册表CAS、规范程序、两对象存储容量、完整一次性动作gas后才提交两对象与`storage_used`。成功返回真实程序hash，活动根用自身原生`Sha256`/`HexEncode`生成的hash核对回执，之后才有偿Exec新工具和完成P2。若旧registry不存在，研究者预定的CAS0首次Conflict后，活动VM最多一次None重试；若registry已为v1，则这两次操作均Conflict且不创建新版本。纯VM隔离EXEC中仍只允许既有的只读侧效应，不允许PublishTool提交。

- 世界内核：`crates/world-kernel/src/program_host.rs` 验证+资源账本+提交，`src/lib.rs` 将Action纳入真实tick和审计；`crates/organism-vm/src/lib.rs` 提供类型化WorldCall、严格参数检查，原有128-block/无界循环限制保留。
- 活动S29：`crates/organism-seeds/src/s29_public_transform.rs` 的`s29_atomic_publish_repair_root`继承原第一次/第二次公开样例发现错误的有界修复算法，改为一次`PublishTool`提交，不再由个体两次`Write`且无孤儿清理副作用。
- [真实有偿原生因果测试](../../crates/organism-seeds/tests/s29_feedback_repair.rs)中`s29_atomic_publication_native`采用三臂真实活动VM、另一个内核负对照臂。三条活动根具有同一程序hash `1c5b7f793793c4b8bdf4b45861f5a13fa5ce50fb438c8e0aa04b7352a3e19a8b`，世界P2 seed42/world seed129、A12000 per epoch、最大16384 ticks。所有新Len程序hash `7a48ca2633bc9429c65eab3c0e9c459be956f88e4fb2176e510a9e5fb29a1496`；研究者提供的首例碰撞次例错误ByteAt(Bytes,1)旧程序hash `6820d1219dd6fc5619ffa4244289cc27394e6811149eb7ca81168fe99ec8774b`，不是P1自主发现的原语。测试核对活动VM真Exec旧工具3、2而第二公开标签为4，真实第二次付费Read后选择Len，核对新版本规范字节与活动`PublishTool`的实际行动输入。

| 冻结组，实际Run #45119834399 | REPAIR | STALE | FRESH |
|---|---:|---:|---:|
| 活动先前registry | 旧ByteAt v0 | 旧ByteAt v1 | 不存在 |
| 原旧工具对两例真EXEC | 3，2（第二错） | 3，2（第二错） | 无 |
| 活动PublishTool真实反馈 | **Some(0)→Ok，收费25 A** | **Some(0)→Conflict 0 A；None→Conflict 0 A** | **Some(0)→Conflict 0 A；None→Ok 25 A** |
| 个体用旧`Action::Write`发布版本/registry | **0次** | **0次** | **0次** |
| 新Len程序对象 | 新安装，registry v1正确引用 | **不存在，无孤儿** | 新安装，registry v0正确引用 |
| `storage_used` 起始→结束 | 101294→103511 B | **101294→101294 B** | 100951→103232 B |
| 实际新Len纯VM Exec | **3,4,5** | 0次 | **3,4,5** |
| heldout输出/外部真值 | 5/5 | 无输出 | 5/5 |
| P2实际结果/独立真值 | 1156832460/同 | 无输出 | 1156832460/同 |
| 实际 B gas | **20** | **0** | **20** |
| VM收费A gas | 17402 | 564 | 17169 |
| 世界Action收费A gas（含初始化） | 6998 | 6367 | 6960 |
| **实际 A 总gas** | **24400** | **6931** | **24129** |
| 世界终止step | **2056** | 15 | **2051** |
| 最后活动VM返回 | U32(1) | U32(0) | U32(1) |
| InvalidCode/Denied、未授权重启、snapshot不一致 | 0、无、无 | 0、无、无 | 0、无、无 |

**FAILURES内核对照**：真实世界`Action::PublishTool`对受限registry路径外的操作返回`Code::Denied`，非规范Program字节返回`Code::InvalidCode`，不足存储返回`Code::Quota`，本epoch A可用gas低于整笔收费返回`Code::Gas`，注册版本预期不匹配返回`Code::Conflict`；这些失败路径均断言目标新版本不存在、registry未创建/篡改且actor `storage_used` 不变。单独验证同一actor已拥有完全相同的内容寻址不可变Program可被合法重复引用，registry CAS成功而仅实际收6 A gas，避免重复存储；全体快照恢复的状态哈希一致。对照中新Len完整发布收费25 A（按两次旧Write费用总和），但不等于整个世界操作无需VM运行/等待成本。此组仅为世界内核动作级不变量，**不是个体自主发现**。

## 解释限制和后续门槛

此结果证明**研究者提供一个通用原子动作时，真实活动VM仍能自主在已给定三原语内根据双公开标签的反馈选择/改写，并以正确CAS采用，失败不遗留未采用版本**。旧合法版本仍作为内容寻址历史保存在世界，成功升级并不意味着自动清理旧版本；能否安全回收未引用对象、跨actor隔离、并发读取/异步任务持有旧版本、容量变化后的更新/快照重放与更长生命周期，均需另一个专门协议。它不是整个世界tick的事务回滚，不是凭空新增个体推理能力，也不是内生生命过程。

**重要可比性**：此版本增加原 E001 六原语之外的受限`PublishTool`，因此已经改变世界允许的能力集合；当正式研究E001时必须对`world_kernel_version / vm_version / action_cost_table / initial_capability_manifest`独立版本化冻结，不能把新S29与旧六原语世界的表现直接比较并归功于学习。三臂研究者导入内容不同、初始存储及genesis不同，同样不能用A总gas差声称等起点净效应。未开展预注册30seed配对、真正开放式算法发现或长期自组织。G0 GO、G1/G2 NO-GO，E001仍未校准/预注册。

本次定向CI已恢复workflow_dispatch-only；正式CI回归不得冒用这一个研究的4/4代表全workspace测试通过。
