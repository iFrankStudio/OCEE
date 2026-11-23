# DEV — 受费的保守程序版本回收（执行前冻结）

**状态：先冻结协议，后补录红绿CI与最终原生4/4证据。** main基线 `d7e8ca5e03f409a301d87446f34da6d921d06175`。继承 [S29有界反馈修复与原子PublishTool原生4/4](./DEV-S29-ATOMIC-TOOL-PUBLISH-PROTOCOL.md)：此前成功升级的旧合法 `/self/versions/<hash>` 会继续占用个体持久存储，即使Registry已替换；单次`PublishTool`仅解决**失败时不创建孤儿**，并不自动回收此前合法安装的版本。

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

## 2026-11-23 — 原生两次红例之后最终4/4：受限版本引用保护、实际回收与费用

**最终原生 CI [#45121695557](https://github.com/iFrankStudio/OCEE/actions/runs/45121695557) @ `bd8a1bf46f0c494142c8252af810c3a2a1d53f39` 已确认 `completed/success`，四独立Rust matrix jobs各 1 passed / 0 failed。** 核心世界、VM与有界S29根源码在最终验证前已提交；严格VM MAX_BLOCKS=128、A12,000/epoch、P2 world seed129/protocol seed42、16,384 tick根限未被放宽。维护程序为研究者预先构造的只含已授权普通VM原语的确定性有限图，由测试经**真实世界付费Read("/sys/active-version")**获得现实已推进的版本，再真实CAS更新 `/self/active` 后在原生`tick_autonomous`内发出`WorldCall::ReclaimVersion(old_hash)`。这一测试阶段切换是研究者操作，不是个体自己产生的维护动机。

受限`ReclaimVersion`新增至`crates/world-kernel/src/program_host.rs`、`src/lib.rs`及`crates/organism-vm/src/lib.rs`。内核真正核验候选哈希格式、owner与现存Program规范哈希，保护初始恢复根、任何活动程序和continuation根；逐项保守扫描**其他**世界对象的key+原始bytes以及消息队列完整原始ASCII hash指针。为允许维护根把拟删hash作为动作常量，调用者自己的当前活动Program版本是**唯一排除raw-string扫描的对象**；不能将此特例误称为完整防止维护程序之后再使用同hash。历史audit保留但不视为活动引用，所有候选若仍被另一个`/self/tools`或显式pin引用均Conflict。计算完整scan+remove A费、检查余额成功后才删对象/退还恰好实际Program规范字节数，失败不改变storage_used。

| 冻结原生arm | 确认结果与付费审计 |
|---|---|
| **POST_REPAIR** | 真实S29原子修复任务成功并在B侧实际扣20 gas；研究者阶段切换后活动维护VM真正回收旧 `ByteAt(Bytes,1)` 版本hash `6820d1219dd6fc5619ffa4244289cc27394e6811149eb7ca81168fe99ec8774b`，动作`Code::Ok`、**确切释放279 bytes、回收实际计费1646 A**。旧版本之后原生Exec变NotFound，新的已注册Len hash `7a48ca2633bc9429c65eab3c0e9c459be956f88e4fb2176e510a9e5fb29a1496` 仍真Exec `[18,2,7,4]`→`U32(4)`；B扣费仍20不重复，最终step2063，总VM A17424+Action A8719=**26143 A**（含研究者全部付费准备/阶段切换与核验Exec）；snapshot状态哈希一致 |
| **REGISTERED** | 同样真实S29成功B20，但研究者通过合法有偿Write增加第二注册表指向旧hash，维护VM第一次请求真实`Code::Conflict`，存储和版本不变。只有真实CAS将第二注册表改指已安装Len后，通过第二次付费切换维护程序再请求，才真实`Code::Ok`，**退还279 bytes**，P2既有B20仍在。不能通过仅更改测试宿主变量来绕过内核持有引用 |
| **ROOTS** | 世界活动Program版本、`initial_root_hash`保留的初始根（即使研究者付费切换新活动程序）、当前维护Program都无法被回收（均`Code::Conflict`）；非规范hash`InvalidCode`，不存在程序`NotFound`；这些拒绝不删除对象、不破坏初始恢复根，snapshot一致 |
| **OPAQUE_REF** | 另一个普通私有pin对象包含完整候选64字节ASCII hash时真实`Conflict`；真实成功SEND将同哈希放进尚未送达消息时仍`Conflict`；合法CAS将pin正文更新为不含该hash、且没有相关消息的独立世界，回收 `Code::Ok`，退还 **277 bytes**、实际收费 **9 A**；A预算归零时真实`Code::Gas`，旧版本与`storage_used`不变，所有快照一致 |

### 两次失败记录与后验改动（不得追认为盲测成功）

- [首次 #45121234194](https://github.com/iFrankStudio/OCEE/actions/runs/45121234194) @`9f43a4e2`：四独立 jobs 全部**编译红**，新内核局部变量名错误使用撇号 `requester's_active`，Rust在解析阶段报错，没有进行任何行为实验。仅将标识符更名为`requesters_active`后重新定向执行；未改变内核回收语义/研究seed。
- [第二次 #45121401232](https://github.com/iFrankStudio/OCEE/actions/runs/45121401232) @`ded797bf`：ROOTS和OPAQUE_REF两组原生通过，**2/4**。POST_REPAIR和REGISTERED的真实P2任务跑完后，研究者维护根切换夹具假设 `/self/active` 一直是v0，但S29 P2此前已经发生合法活动根版本更新；世界正确返回 `Code::Conflict`，测试断言失败。该红例说明“CAS必须以环境实际当前版本为准”，不能把测试的固定版本假设强加给内核。
- [最终 #45121695557](https://github.com/iFrankStudio/OCEE/actions/runs/45121695557) @`bd8a1bf46f`：夹具先付费Read已有`/sys/active-version`获得真实版本，再据此CAS切换研究者给定的维护程序，其余规则、内核受保护引用扫描、有限世界和实验seed不变。四独立 jobs完整4/4成功。

### 严格资源与科学结论

**安全回收不等于经济上合算。** POST_REPAIR针对仅**279B**旧程序，需要保守扫描其他世界持久对象/消息，动作收**1646 A gas**。A gas与存储bytes不能直接换算，缺少长期容量压力和收益评估时不能把回收写成“提升净效用”；不过回收确实减少存储占用且新工具未损坏。与OPAQUE_REF仅9 A的差异主要反映世界状态大小不同（含完整S29根及工具），不是算法本体差异或等起点性能因果比较。世界本次仍只有单actor实验；对其他协议任意格式的二进制隐式引用、跨世界分布式租约、暂停的外部任务与静态内容哈希在无内存扫描场景下都**没有完全引用证明**，调用者自身活动Program字节码有意跳过字符串扫描的例外也应披露。删除成功后可能再次按相同内容hash合法安装原程序，这不是历史审计回滚。研究者定义这个第8类受限原语，属于世界能力扩展而不是个体自发创造维护策略；正式E001六原语基线及版本/成本必须独立冻结，G0 GO/G1/G2 NO-GO、E001未校准/预注册。

下一最值得检验的不是堆更多固定GC测试，而是**受保护引用索引、租约/代际/增量扫描能否在不引入错误阴性时下降1646 A这一真实高成本**，配对同一世界快照和可证伪的过期索引/异步消息负对照；否则“为了省少量存储而消耗过多算力”可能使维护不适合高压力资源期。
