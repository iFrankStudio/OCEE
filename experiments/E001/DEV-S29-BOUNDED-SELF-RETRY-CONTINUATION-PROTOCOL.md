# DEV — 单次激活的有界连续维护重试：失败记忆、VM续执行和世界tick

**状态：研究设计先于代码冻结；已附首次原生3/3正负结果。** 主分支基线 `dab64fd7702c2fba83d0b8fb0cfc8de0c8661aef`。前次[持久反馈记忆](./DEV-S29-PAID-PERSISTENT-MAINTENANCE-BACKOFF-PROTOCOL.md)首轮原生3/3，但每一个维护episode仍是研究者通过付费Read`/sys/active-version`再CAS`/self/active`重新启动同一根。当前内核`tick_autonomous`与VM已有`Continuation.loop_counters`持久化、`Terminator::RepeatBounded`静态上界256；这次**不修改任何kernel或VM接口**，只将运行计划保持在同一个尚未Return的程序中。

## 冻结机制和科学含义

研究者仍在完成真实S29错误ByteAt二公开例修复/原子Len发布/heldout5/P2任务B20之后，**仅一次**付费安装并按真实active-version CAS激活一份研究者提供的有界维护根。候选旧程序仍是三个已合法安装内容寻址版本 `[279,276,279]B`，真实world seed129/protocol42、A每epoch12k、原16k tick任务上限，不更改能力、指令、引用索引、状态可见面或计费规则。

在旧`feedback_backoff_root`基础上将入口改为 `entry -> retry_header(RepeatBounded limit:3) -> risk_entry`，第三轮后的 exhausted 直接Return U32(0)。各轮内逻辑与之前完全一致：付费Read自身`/self/maintenance/risk-s29`，无对象或 `b"0"` 则付费Read真实`/sys/resource-status`，压力+预算满足时真Index+最多三次GC；首个真实`ReclaimIndexedVersion`返回Conflict则该VM用普通CAS None真正Write风险`1`；下一轮读到`1`则用普通CAS0真正Write风险`0`、**不建索引也不删**；随后重新允许资源条件分支。

和前次不同，所有失败/冷却分支不立即终止：它们需经一个实际`WorldCall::Observe()`（普通收费1A、真实一个world tick），再转回同一个静态`RepeatBounded`头；没有研究者重新激活、外部Tick故意重置continuation、无隐藏host计时器、无无限循环。所有写/读都来自真实活动VM，且每个已执行循环和已排队反馈保存在snapshot里。需要同时记录至少一个独立真实Observe与连续的世界step；**不声称跨过整个1024tick epoch或产生自主睡眠调度**：只有同一活动VM后续tick的有限继续执行。

## 三条预先固定的真实原生验证

1. **ONE_ACTIVATION_RELEASE**：研究者正常有偿设置世界高存储压力与足够A，实际连续`tick_autonomous`；在第一轮VM成功`IndexVersionRefs`后，研究者通过真实CAS Write添加首旧hash pin，使首次GC真实Conflict，VM自己收费Write`b"1"` v0；不重启根，VM经真实Observe前进到第2轮，VM真实Read风险1、付费Write`b"0"` v1，**第2轮不得Index**；在第2轮写0完成后研究者通过真实CAS同长度换掉pin正文，仍不重启根；第3轮VM自身在新的tick读风险0，重新收费索引并连续真GC3×Ok，释放**834B**，最后Return U32(1)。需审计仅一次研究者初始`/self/active`切换、三轮中零次该路径Write、每轮真实风险Read/Write、每个retire Code、原B20不重复、新Len真Exec、状态hash及快照。
2. **PIN_STAYS**：相同根与真实第一轮竞态，但研究者**始终不释放pin**。第二轮同样真实短期退避不扫描，第3轮会受限制再次真Index、首GC仍`Conflict`，在有界原型中`risk-s29`现在v1但失败处理只支持CAS None，额外风险写`Conflict`（不得假报记忆成功），第3轮后RepeatBounded预算耗尽安全Return U32(0)，目标三旧版均未删，B20/snapshot不变；明确说明“第三轮CAS不适应多次冲突”是实际限制，**不**后验修成支持任意版本写。
3. **SNAPSHOT_CONTINUATION**：在第一轮真实GC Conflict且VM确实已普通Write风险1后，复制完整同一world`snapshot_json`两份（包括`vm_continuation.loop_counters`、pending_feedback、已完成世界审计），各支仅运行同样普通世界tick，不再Read active-version/CAS重启；两支在相同step真实Write替换pin、恢复运行并完成相同第2轮冷却和第3轮成功删除，最终state_hash、审计轨迹、实际A/B支出完全一致；不能由宿主重建新Continuation代替恢复。

如有原生红例，记录真实报错与最小修复，不改变本协议核心条件/上限/seed。只设置一次限定scope GitHub Actions 工作流，不跑全项目老Study；最终完成后改`workflow_dispatch` only。

## 解释边界

通过仅代表**研究者编写的固定有界Program在单次活动启用后的运行时条件行为、真实错误回执持久记忆与有限自我重试**。环境写入并撤销pin仍由实验研究者控制；世界的自动`tick_autonomous`也由外部模拟驱动器逐tick调用；候选/计划/固定一轮退避/目标均是预置，不代表生物式自发目标、独立时间调度、任意长期风险学习或开放式演化。保持G0 GO，G1/G2 NO-GO，E001仍未预注册/校准。

## 2026-11-23 — 有界同一VM continuation跨多tick自主重试：首次原生3/3，无新增kernel/VM能力

先以commit `03853769509c349087464471b6e368075bd96fe9` 将本协议**在实际代码之前冻结**，再仅在 `crates/organism-seeds/tests/s29_reclaim_version.rs` 中添加研究者提供的静态根 `self_retry_maintenance_root`、真实世界夹具和限定Rust CI，机制提交 `6468a250b67c8b7b3c947ca6a6aa82de3ed3059e`。未修改 `crates/world-kernel`、`crates/organism-vm` 或 A/B 定价/ACL：复用原有可校验 `RepeatBounded(limit=3)`、持久 `Continuation.loop_counters`、`WorldCall::Read/Write/Observe/IndexVersionRefs/ReclaimIndexedVersion`。所有运行均先真实完成S29 wrong ByteAt第二公开例发现并原子发布Len，再完成heldout5和真实 P2 world129/protocol42 FOLD32 B计算B20，之后研究者**仅一次**经有偿`Read(/sys/active-version)`与普通CAS激活研究者提供的维护根。根哈希所有组相同：`dfdd293c2602cc68c709d40dfa447e19620e314c8ed16114c771290e2a1114c0`。

**首次 native [#45141256292](https://github.com/iFrankStudio/OCEE/actions/runs/45141256292) @6468a250b67c8b7b3c947ca6a6aa82de3ed3059e completed/success：三独立 Rust jobs各1 passed/0 failed，无本轮红例。** 三臂实验严格检查维护根第一次激活之后`/self/active`世界Write发生次数**0**，每次失败/冷却重试均通过普通有偿`WorldCall::Observe()`（而非宿主重新激活）进入下一个实际world tick；固定静态上限最多3轮，其计数受现有VM verifier控制。

### ONE_ACTIVATION_RELEASE（真实序列与资源账本）

| 运行状态 | VM和环境发生的真实事件 | 世界动作/返回 |
|---|---|---|
| 1：环境冲突 | 活动VM真索引；研究者在此后用普通有偿Write给某旧版本增加原始ASCII hash pin，证书被合法写屏障失效；VM真实回收收到Conflict并普通CAS None付费写`risk-s29=b"1"`，v0 | 索引一次，GC `Conflict`，未删 |
| 2：有界退避 | 同一尚未结束的VM以收费`Observe`跨入后续tick，真实读取自有持久风险1，按研究者写死的单轮退避策略不请求索引，普通CAS0付费写风险0，v1 | 索引0次，GC0次，无版本删 |
| 3：恢复尝试 | 研究者在第二轮写0之后通过正常有偿CAS把64B pin正文改为不再包含hash的等长文本；**不触碰`/self/active`**；同一连续VM重新读风险0、真实资源Read并付费索引、依次GC | 索引一次，三次 `Ok`，释放 **834B**，返回`U32(1)` |

实际**仅维护根首次激活后没有任何再次Write(/self/active)**，有偿普通`Observe`次数 **2**（两个失败/冷却路径）；真实索引次数2；`ReclaimIndexedVersion`审计序列 `[Conflict,Ok,Ok,Ok]`；三旧候选实际已删 `[279,276,279]B`合计834B，已采用正确Len工具仍在registry。开始执行前已付费完成fixture的 `storage_used=112006B`，环境真实新增pin64B + VM真实新增risk对象1B，最终 `112006+64+1−834=111237B`；原先B20无再次调用，世界step **2079**、全历史VM A **17806** + world Action A **10931**（包括S29之前全部工作、研究者安装根、外部pin写与真正GC等；**不能全部归因于维护VM的净收益**）；world snapshot roundtrip hash一致。每个yield/read/write/索引/GC均是实际内核执行，不是给VM注入测试回执。

### PIN_STAYS（真实负对照：不能误删且有界停机）

同一个根、同一次首次真实冲突 + risk1写入，**研究者不解除pin**。同一VM经第二轮主动退避写0、第三轮再次真实创建索引后，首个GC仍由内核拒绝`Conflict`，没有删除任何三个已安装的旧版本。已观察真实GC序列 `[Conflict,Conflict]`、索引2次、收费Observe3次；再次将风险写1的研究者旧固定CAS None遇到已有对象v1，内核真实拒绝`Conflict`且0A、不修改risk0；第3轮后RepeatBounded计数到3停止 `U32(0)`，世界step**2078**，B20不重复，snapshot一致。**该记忆写回失败是现存机制上界，不能称连续失败的长期自适应已解决。**

### SNAPSHOT_CONTINUATION（原生确定性重放）

首次真实付费索引、pin竞态、GC Conflict、VM自己付费持久写风险1(v0)后，正在执行的`vm_continuation`尚未Return，`retry_header`循环计数 **1**。从这份世界的完整 `snapshot_json` 构造第二个独立世界，逐tick重复相同自主VM动作，直到第二轮普通风险CAS0（循环计数2），两支均以正常CAS同长度移除pin，再逐tick跑到第三轮三个GC真Ok（循环计数3）。**两支最终完整snapshot_json字节相同**，包括世界状态hash、audit chain、loop counters与未完成反馈衔接；三旧版真释放834B，B20不重复，step2079。恢复程序未被研究者重装、重置或重新激活，这证明持久Continuation能够在本示例的多轮有界行动后确定性继续。

### 科学结论与明确未达成目标

本轮去掉了**每次维护episode由研究者重复写`/self/active`重新激活**的依赖——**只证明同一静态根被首次安装激活之后，现有确定性world每个`tick_autonomous`驱动它自主决定跳过或重试并保证多轮风险状态与循环计数可恢复**。它没有消除“世界外部引擎驱动每一个tick”的存在（这是原世界的物理调度基础），更没有证明独立永续自主唤醒/跨完整1024tick epoch的动态时间学习；研究者仍选定候选3版本、两阈值、退避整轮长度、有限三轮上限与外部pin撤销，未来若引用长期存在则继续碰到旧CAS None失效并有界停机。算法/目标/约束没有发生个体内生变异或开放式演化。本实验无需新增能力，但依赖此前已扩展的付费GC和资源观察接口，不能混称正式E001六原语不变。**G0 GO、G1/G2 NO-GO，E001仍未校准/预注册。**

下一轮研究应对准**长时间可验证的资源/失败历史与动态条件下的重试间隔**：测定跨epoch/长序列真正的索引失效概率和维护收益，克服已存在risk对象v1后CAS None无效（需先确定可信版本来源，而不是无偿突破CAS），以及更重要的**候选发现与策略目标如何不再由研究者预置**。不要将有界循环下的成功当作自主产生维护目标。
