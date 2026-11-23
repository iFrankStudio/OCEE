# DEV — 自身对象原子内容+版本读取与多次GC失败记忆CAS（先冻结）

**状态：实验设计先于代码冻结；首次原生4/4完成并归档。** 基线 `b243990f60bf5d44432d434895bd47cae6826b9d`。紧接[同根单次激活三轮retry/持久Continuation](./DEV-S29-BOUNDED-SELF-RETRY-CONTINUATION-PROTOCOL.md) native3/3：原世界`Read("/self/maintenance/risk-s29")`只有原始正文，不返回`Object.version`，上轮VM写b"1"时预置CAS None只在首次缺失成立；第三轮再次真实GC Conflict后对象已v1，记忆写Conflict而无法延续动态历史。不能将失败CAS当作成功学习；需要真实有偿观察版本，不能无偿偷读内核版本。

## 预冻结新增能力及安全不变量

不增加WorldCall/action种类、不动VM指令与收费GC算法；**唯一代码新增的是既有`Action/WorldCall::Read`的一种系统只读路径** `/sys/self-object/{relative}`，其中`relative`必须为不空且以`/`分隔的规范片段（没有空、.、..段），最终只能映射到当前合法actor已拥有的普通`/self/{relative}`对象。严禁跨actor、任意`/public`、`/sys`、根外键、路径遍历查询。真实不存在时`NotFound`（不需提前收费），未授权时`Denied`。返回`ActionResult.payload`为严格JSON `{"version": U32, "bytes": [u8...]}`，同时读取自有对象**原子同一快照版本与正文**；不会暴露任意非自身机密。版本超过U32则`Quota`而非截断。使用`gas = 2+ceil(byte_len/64)` A正常付费；与旧普通Read返回原始bytes兼容，不修改它。对潜在巨量正文，实际VM可见响应仍遵循旧MAX_VALUE_BYTES限制；实验只用1字节状态。新增了正式E001基线所无的**自身CAS元数据可见面**，将来须版本化，而不是声称原实验环境不变。

## 冻结有界同一活动VM策略（不改已有初始P2世界）

S29 world seed129/P2 protocol42，a_gas_per_epoch12000，原实际P2任务执行完B20，后研究者仅一次付费安装三未引用合法版`[279,276,279]`B和同一有界维护根，普通付费Read`/sys/active-version`后CAS激活；此后本轮**没有任何新的`/self/active`切换**。根继续使用 `RepeatBounded(limit:3)`、每次失败/冷却通过`WorldCall::Observe()`付费真实world tick。每轮读取风险改成`Read("/sys/self-object/maintenance/risk-s29")`：`NotFound`表示首次尝试；`Ok`原子提取`bytes`和`version`，只接受`b"0"`、`b"1"`；未知字节、不匹配类型或不可表示版本均安全退出，不做GC。

- `b"1"`（上一次真实GC失败）冷却一轮、不索引不回收，并用刚刚观察到的**真实版本号**有偿CAS Write b"0"；只有Write真实`Ok`才进入下一次有界循环，`Conflict`必须立即终止不伪造记忆成功。
- `NotFound`/`b"0"`且真实空间压力与A预算满足时，实际建一次引用索引、尝试三个已知候选；首次实际`ReclaimIndexedVersion`为`Conflict`才认为“这次索引可能浪费”，再**有偿重新读取自身风险对象当前真实bytes+version**（不能沿用旧值，允许外部并发写）。如果不存在，真正CAS None新建b"1"；如果存在且正文合法，则用刚读到的`version`真正CAS写b"1"；如果读/写出错，安全停止不重试GC。此机制**不预设v0/v1/v2**，但不保证每次并发冲突必定成功，真实stale-CAS必须fail-closed。
- 成功删除三份返回U32(1)；始终受静态三轮上限，不加入无限自举/自我复制；候选集/目标/起始维护根/容量压力阈值25%和A预算2200仍然由研究者给定。

## 预冻结四臂

1. **REPEAT_CONFLICT**：第一次活动VM真实Index后，研究者正常付费Write pin某旧程序hash→索引失效，GC真的Conflict，VM自己元版本Read NotFound并付费Write risk1对象v0；第二轮自读`{v0,b1}`，无Index，CAS0更新risk0 v1；研究者**一直保留pin**，第三轮重建索引后GC再次真Conflict，VM自己付费`Read`得到`{v1,b0}`，CAS1真正更新risk1 **v2**；三轮封顶安全返回U32(0)，三个旧版保留，B20/snapshot一致。原上轮预期重写失败的负例必须被这里转成真正成功状态更新；要列出每次Read、Write的真实收费。
2. **STALE_CAS**：和1同场景直到第三轮真实GC Conflict后、个体收到自身状态Read版本v1；研究者在VM下一次Write行动前先用普通世界合法CAS将risk从v1改为某个有效但**不同**的1字节状态（如b"1"）使版本递增v2；VM随后使用所见旧v1进行Write b1，内核必须真实`Conflict`（不更新v2，不继续GC，不谎称记忆写成功）；无误删、B20及快照一致。测试者不能直接改`WorldState.objects`伪造冲突。
3. **RELEASE_AND_REPLAY**：先真实首次Conflict+记忆b1 v0，按2nd episode自动冷却b0 v1，再由研究者合法付费CAS清除旧hash pin；第三轮同根VM重试成功，GC3×Ok释放834B，b0 v1保持，B20不重算；在真实第2轮前后复制完整世界快照，双份继续相同付费tick/普通CAS，最终状态、审计、VM历史/版本一致。应保留此前一次激活保证。
4. **METADATA_GUARDS**：实际`Action::Read`读现有自身1字节记忆，验证`version:0`及`bytes:[49]`，真实付费2+ceil(1/64)=3 A；普通`Read("/self/...")`仍返回原始1字节不变；非法路径、未拥有对象、缺失对象、版本越界应明确安全拒绝（若需受信测试夹具模拟版本极限则不能误称真实actor自行提升版本），不存在绕过Write CAS版本逻辑或擅自读取他人/公共对象的能力。

三组长臂仅使用限定Rust CI runner、真实旧S29/P2工作流、快照一致且不重复B20；所有红例原样保留，若修正仅限合法类型/夹具，绝不可放松研究前冻结判据。最终定向 workflow 调回`workflow_dispatch` only。

## 科学边界

本轮若通过可证明**有界单次激活 VM 能多次记录实际维护失败、按真正当前版本更新普通自身状态，并在并发写入时拒绝覆盖**；但记忆位和退避长度、候选版本、空间/预算阈值、初始激活与测试环境pin均研究者预设，不能推断自发产生长期目标/风险估计、完整epoch自主时钟、全局GC引用安全，亦非开放式演化。G0 GO / G1、G2 NO-GO；正式E001仍未预注册/校准。

## 2026-11-23 — 真实自身对象版本观测修复多次GC失败记忆CAS：原生首次4/4

研究设计在代码修改前通过 [`11f4548021e1128109c77f4e89946d38e5286613`](https://github.com/iFrankStudio/OCEE/commit/11f4548021e1128109c77f4e89946d38e5286613)预冻结。代码`crates/world-kernel/src/lib.rs`仅在原有`Action::Read`中增加`/sys/self-object/{relative}`（且不改变旧普通Read返回、WorldCall类型和vm verifier），由合法actor访问**自己已拥有的`/self/{relative}`**，严格拒绝空、.、..路径段、外人owner和超过U32版本号；以**同一内核对象**的body和CAS version返回`{"bytes":[u8...],"version":u32}`，真实收取`2+ceil(len/64)` A。不存在对象返回NotFound，未授权Denied，版本过大Quota；该新观察面是**研究者引入的开发扩展**，不是正式E001六原语不变性证明。过去失败是旧`Read(/self)`只有bytes，VM首次CAS None→v0、退避CAS0→v1，再次Conflict仍错误用CAS None而失败；此次改为现有VM语言JsonFieldBytes/JsonFieldU32解析真版本，再付费`Write`使用读取到的当前版本，CAS竞态仍拒绝不覆写。

**[原生 Actions #45145971582](https://github.com/iFrankStudio/OCEE/actions/runs/45145971582) @ `b6682deab43b86aa1835d2b6de16e1027445d412` 首轮即 completed/success，独立 Rust 四个jobs各1 passed/0 failed，没有本轮红例。** 测试和维护根内容是同一研究预置、受VM verifier约束的有限三轮程序，hash全部长臂相同：`88af07516c0e101b260b7950f0e1de5afbfeb5e556952b4bd26b26a2be306815`。三个长臂均先使用真实S29错ByteAt（第一公开例碰撞、第二例失败）、后VM原子发布正确Len、heldout5、真实P2 world129/protocol42的FOLD32结果，B费用**20**；接着研究者有偿安装3个合法旧工具`[279,276,279]B`和本次根，真正`Read(/sys/active-version)`及CAS首次启动；后续再无任何根切换，`tick_autonomous`使同一持久`Continuation.loop_counters`的`RepeatBounded(limit3)`驱动每轮有偿Observe与重试。不变A12k/epoch、本来GC收费、活引用安全检查、真实空间压力/2200 A预算门槛与三个预先给定候选。

### 四臂原生审计及真实账本

| 原生arm | 世界行为与VM反馈 | 有偿索引/旧程序删除 | 持久风险状态 | 其他记录 |
|---|---|---|---|---|
| **REPEAT_CONFLICT** | 第一次付费Index后研究者真实Write pin，GC返回`Conflict`，第二轮VM记忆冷却并跳过索引；第三轮pin仍在，付费索引后GC再次`Conflict`；没有重启活动根 | Index **2次**，GC `[Conflict,Conflict]`，**0B回收** | VM真正写序列 `1(v0)→0(v1)→1(v2)`，三次**Ok、每次3 A** | 核验自身原子meta读序列`[(NotFound),(NotFound),(v0,b1),(v1,b0),(v1,b0)]`，真实成功meta读各3 A；`storage_used=115049B`、B20、累计VM A17960，world Action A11096、step2080（包含前置S29与研究者夹具） |
| **STALE_CAS** | VM第三轮真实GC Conflict后元数据Read到`v1,b0`；在VM下个Write之前，研究者**合法付费CAS**将risk改`b1,v2`；VM仍以旧v1写b1遭真实`Conflict`，立即保守退出，不盲目再次索引或误报成功 | Index **2次**，GC`[Conflict,Conflict]`，**0B回收** | 外部更改后的记忆仍是`b1,v2`，VM过期Write`Conflict`且**0A** | 无活引用误删、B20、root只在开头激活一次、snapshot一致、step2080 |
| **RELEASE_AND_REPLAY** | 首次真实Conflict+风险b1/v0后，按完整世界快照复制两支（包括pending VM反馈），第二轮VM读1跳过索引并付费写0/v1；研究者在两支均以合法同长度CAS移除pin，再继续同VM第三轮重试 | Index **2次**，GC `[Conflict,Ok,Ok,Ok]`，释放三个旧版 **834B** | 记忆仍b0/v1 | 两支最终**完整snapshot及audit逐字节相同**，无需重启根；最终`storage_used=114215B`、B20、VM A17911、Action A11119、step2080（均含前置任务+夹具开销） |
| **METADATA_GUARDS** | 在纯World::new的小范围权限定向实验中，普通actor有偿读真实`/self/maintenance/risk-s29`一字节`b1`和版本v0 | 1字节元数据读取收费 **3 A**；旧普通Read仍仅返回`b1`原文 | 非法路径严格BadRequest；缺失NotFound；跨owner Denied；版本超过U32 Quota | 所有检查通过。构造外人owner和超大version边界需**测试驱动器直接编辑可信fixture**，不能声称这些异常由活动生命体自己创造 |

REPEAT_CONFLICT实际可见原生`meta_reads=[(NotFound,None,0),(NotFound,None,0),(Ok,Some((0,[49])),3),(Ok,Some((1,[48])),3),(Ok,Some((1,[48])),3)]`；与普通风险写`[([49],Ok,3),([48],Ok,3),([49],Ok,3)]`相配。区别于前次“第三冲突写CAS None失败”的确切缺陷，本次**第二次真正Conflict后已成功记录到v2**；没有删除仍被pin引用的程序，三次循环后安全Return0。这是可验版本一致性改进，而不是人为擦除冲突状态。

### 解释边界与待解决事项

- 元数据读和实际CAS写之间仍然存在竞态：**STALE_CAS原生验证了正确拒绝**，而不是假称“版本读就使CAS永远成功”。VM对写失败安全停止，在没有回退准备/再次版本Read的情况下不会自动解决任意高争用。
- 读状态与写入风险的普通1B对象、GC两次扫描及主VM指令均要付费。三轮成功删除的834B相对于多数扫描A成本依然不能被宣称有跨资源的净正效益；唯一能肯定的本轮改进是“连续失败后的普通自有风险状态不会再因为旧固定版本必然失败”。不能用这些**事先安排的确定性pin轨迹**推算现实写频、失效概率或长程优势。
- 研究者仍先给定维护目标、旧版本hash候选、25%储存/2200 A预算阈值、三轮退避计划和初始根，世界外部驱动器逐tick调用同一`tick_autonomous`；未跨数个完整epoch建立动态自唤醒/自定目标/开放式演化，也未证明通用引用回收安全（仅安装版本的原始64字节ASCII引用）。
- 已新增`/sys/self-object/`的元信息可见面，在正式E001必须独立冻结版本、ACL与成本；**G0 GO、G1/G2 NO-GO，E001未校准/预注册**。下一研究应优先在**跨epoch真实外部写入轨迹**中测量值得维护的触发概率与 A/存储机会成本，而不再堆砌类似的三轮固定成功用例。需要特别研究维护候选能否由个体从其自身账本发现，而非由研究者预先嵌入程序hash。
