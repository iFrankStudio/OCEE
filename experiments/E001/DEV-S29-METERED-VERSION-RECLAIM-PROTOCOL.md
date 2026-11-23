# DEV — 受费的保守程序版本回收（执行前冻结）

**状态：预冻结协议；尚无本轮原生证据。** main基线 `d7e8ca5e03f409a301d87446f34da6d921d06175`。继承 [S29有界反馈修复与原子PublishTool原生4/4](./DEV-S29-ATOMIC-TOOL-PUBLISH-PROTOCOL.md)：此前成功升级的旧合法 `/self/versions/<hash>` 会继续占用个体持久存储，即使Registry已替换；单次`PublishTool`仅解决**失败时不创建孤儿**，并不自动回收此前合法安装的版本。

## 核心问题与明确不保证的范围

研究**确定性单世界、单个动作 tick 内**是否能安全回收不再被已知持有者引用的旧合法纯VM程序版本，计费、保护内核恢复根与已注册执行入口，并保持可重放。未知外部网络、任意编码的隐式引用或尚未引入的多actor分布式租约不属于本轮保证；删去一个版本无法自动撤销历史EXEC，也不能把回收写成整个世界可回滚。个体并没有在本研究自主产生“应何时GC”的目标。

## 冻结动作与严格约束

新增 `WorldCall::ReclaimVersion(hash:Bytes)`（单参数）与 `Action::ReclaimVersion{actor, program_hash:String}`，只允许 **本actor合法拥有的** `/self/versions/<64-byte lowercase hex>`；非规范哈希返回InvalidCode，缺失对象NotFound，其他owner Denied，损坏的规范字节/内容哈希 InvalidCode。不会删除普通私有数据、注册表、任意公共对象、活动根；`/self/versions` 规范程序仍需通过完全相同验证。

单次 tick 执行如下保守阻止条件，至少包括：
- 任何 `state.organisms[*].active_program_hash`、正在运行的 `vm_continuation.program_hash`、以及 `state.initial_root_hash` 的同hash拒绝；不能删除当前正在运行或供重启恢复的根程序；
- 除目标版本对象外的世界内持久对象（**包括任何其他程序版本、/self/tools、/self/active、显式/ self/pins 及public对象**）的**原始bytes中出现完整64字节ASCII hash子串**，以及对象key中的完整字面hash，均保守拒绝；避免只是过滤某一个特定注册表而漏掉第二引用。为允许活动个体显式发出回收指令，**唯一例外**是当前请求者的`active_program_hash`对应的规范Program对象：其静态常量允许携带“拟回收hash”作为请求参数。该例外不保护该根程序将来仍试图使用旧hash，必须明确是调用者的责任。
- 所有`pending_deliveries`和各节点`mailboxes`中的消息原始bytes出现完整64字节ASCII hash时拒绝；在单tick之间没有外部异步代码执行，但其消息可成为将来的读取结果，必须保守保护。
- 历史`audit`不视为存活引用：审计已被独立存档，重放由历史Action从genesis重建，不应该因历史哈希出现而永远不能释放版本；而将哈希转码到任意二进制格式后持久保存，不在本次检查能力内，不可误称全局完全可达性证明。

按字典序确定性遍历其余世界对象、所有pending/mailbox消息，长度扫描范围记`scanned_bytes`（仅实际扫描的字节）；安全成功的单次有偿回收`A gas = 1 + ceil(scanned_bytes/64) + 2 + ceil(program_bytes.len/64)`。先全部检查（包括本actor保有存储额度足够），再从A资源额度一次扣完整gas，成功才删除旧版本并令`storage_used -= program_bytes.len`；失败时不删除、不减少存储、不改变注册/active/continuation，Gas拒绝也不能免费删除程序。无法满足任何保护条件返回Conflict或Denied/InvalidCode，不做删除；与现有Action一致，失败预检期间不另收Action费，但若VM发起仍真实承担VM指令gas。累计扫描空间不超过世界资源/消息配额可容纳对象，测试不得制造无界扫描。

### 本轮原生四臂（冻结）

- **POST_REPAIR**：真实S29原子修复任务完成、B20、heldout正确后，研究者付费切换到公开审计的**固定维护根**（这是研究者给出的阶段切换，不是个体自行产生管理动机）。维护根通过真实活动VM `WorldCall::ReclaimVersion`请求刚被替换的旧`ByteAt(Bytes,1)`版本。应实际删除并释放**恰等于旧规范程序字节长度**的持久空间；新注册Len不变、仍能Exec得到正确结果，快照一致；审计明确记录维护根与旧工具实际hash及回收费。不得改动历史B结论。
- **REGISTERED**：相同已完成的S29世界，但另一个 `/self/tools/second` 注册引用旧hash；维护根真实ReclaimVersion应Conflict且旧程序/存储不变。移除注册表不能伪造：仅用合法CAS写其他有效版本替换此第二引用之后，才允许后续另一次合法申请（可在同组通过新维护根发起）回收成功。
- **ROOTS**：独立内核Action负对照：尝试删目前活动程序和初始恢复根必须Conflict；合法执行中的程序如果被一个尚在进行的 continuation使用，即使活跃入口已切换也不能被回收；外部owner/noncanonical/NotFound必须拒绝，不改变存储/注册/运行状态。
- **OPAQUE_REF**：真实世界内核Action负对照：在另一个私有对象或消息bytes里保存完整ASCII候选hash，回收应Conflict且存储不变；在已知持久引用中移除或改变hash后应允许收回；任意编码被转码的隐藏引用明确**不在保证之内**。额外`Gas`不足测试必须证实旧程序仍存在，成本被审计。

严格不提高S29 P2 world129/protocol42 A12000/epoch、原生最大16,384 ticks、VM纯隔离执行与程序静态128块禁令。不变更旧六原语/已有`PublishTool`动作旧语义。原生只跑定向矩阵，保存首轮红例，验收后改为手动工作流。实际记录旧版本hash、对象存储字节、真实回收Action和Code、A_VM/action费用、世界step、所有引用的前后变化、B不重复计费、snapshot重放一致与错误路径无副作用。

**科学边界**：这是研究者提供的第8类受限世界能力 `ReclaimVersion`，相对于正式E001六原语与此前开发扩展，必须在未来正式E001校准及预注册前冻结版本和能力声明；不是个体自主学会GC策略。G0 GO、G1/G2 NO-GO，E001尚未正式预注册。
