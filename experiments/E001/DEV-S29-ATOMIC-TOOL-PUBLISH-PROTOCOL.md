# DEV — 可计费、内容寻址、原子工具版本发布与注册 CAS（运行前冻结）

**状态：协议先于实现冻结，原生CI未运行。** 基线 `f44f1023c555d695d2e4c22ba266f2fd9285d9db`；延续 [S29首/次公开样例反馈恢复](./DEV-S29-SECOND-LABEL-REPAIR-PROTOCOL.md) 原生[#45115636978](https://github.com/iFrankStudio/OCEE/actions/runs/45115636978) 首次4/4：现有新程序先付费写 `/self/versions/<hash>`，再写 `/self/tools/... ` registry CAS，STALE case在失败时留下新版本“孤儿”。此研究解决的是**有限范围的原子提交/存储一致性缺口**，不是新增自由程序权限、任意回滚，亦非自主发明算法。

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
