# DEV — S29 内核认证逐来源引用差分、真CAS写入与旧证书保守对照（执行前冻结）

**状态：研究协议先于实现冻结；首次原生六臂6/6已完成并归档。** 基线 `65bdff70f0c7762f82bf444ab54af64dde11e84d`。上一[严格短写入认证保留](./DEV-S29-CERTIFIED-SHORT-WRITE-PRESERVATION.md)原生4/4证明1B短写可以安全保留原有证书，却在一条旧64B哈希引用被覆盖时仍必须保守拒绝回收，只有第二次真正付费全索引才能解除旧标记；这是**假阳性旧引用**，不是新的活引用安全证明。本轮只改进现有64字节原始ASCII哈希模型，不宣称任意编码/间接引用、分布式租约或开放式演化。

## 安全契约与最小能力扩展

引入显式 `Action::WriteIndexed` 与对应 `WorldCall::WriteIndexed(Bytes key,Bytes value,U32 expectedCAS)`，不能覆盖/修改原 `Write` 的历史费用或旧实验语义。该新动作仅限合法actor持有的普通 `/self/` key，禁止 `/self/active`、`/self/versions/`、`/self/tools/`，严格重用原正常Write的owner、CAS、长度、存储额度、程序路径检查与当前epoch真实A费用；没有当前**本actor/active hash**相匹配、并且具备逐来源记录的内核可信证书，必须`Conflict`且**不更新对象**，不得凭用户可写`/self/index`伪造认证。

修改旧`IndexVersionRefs`在同一次付费扫描中保留**每个被扫描对象key对应的源哈希集合**，以及扫描时**在途消息/邮箱中的原始hash集合**。记录来源集合而非仅union；源内出现多次同hash只算一次。程序自身的版本key中的自hash必须像旧算法一样排除；当前活动root整个对象仍沿用原有免扫描规则。每个原子IndexedWrite成功时，以**同一普通原始64B窗口规则**扫描新对象key和bytes，替换该来源的集合、从其它仍存在来源及上轮消息集合重建安全并集（`referenced`）；由于其他引用性动作仍**全量失效**认证，而原短Write仅能去掉旧引用/无能力新增64B引用，旧的源集合可能多记引用但绝不能漏记。旧快照缺少来源明细时，新IndexedWrite拒绝（不能偷偷把旧全局referenced当成可精确差分来源）。

**付款原则**：`IndexVersionRefs`沿用其真实已经按遍历key+body总字节收费的一次全量扫描，分组信息在同次已付款扫描中提取，不重复偷读世界；新`WriteIndexed`收原Write价 `2+ceil(new_len/16)`，另收显性`1+ceil((key_len+new_len)/64)+N` A，其中 `N` 为证书中全部来源集合当前记录的哈希成员总数（既包含新写入后的来源），覆盖需重新复核的缓存索引元信息，严格在一笔收费前检查溢出和A不足。证书存于kernel内存而不是actor可见`/self/`对象，日志必须报告本次证书**序列化字节长度**，但暂不冒充其内存预算已通过存储配额收费；是否需要独立索引内存配额是未来问题。**一切CAS、容量或gas拒绝必须原子不产生任何修改**。

旧的`Action::Write`短写特例、原注册/active保护、`PublishTool`、Send、Receive、Exec、epoch环境事件，保持历史路径与现有失效规则。快速GC仍拒绝 `referenced.contains(target)`、活动/初始/continuation根；纯证书自身不成为哈希权威，index是内核从真实付费扫描签发。回收成功后仍可保留过保守来源直到重建证书（不允许因回收一件版本就误删其它被引用版本）。

## 冻结的六臂原生判据

世界长臂一律先真实执行S29修复P2 world129/protocol42，B20一次、12k A每epoch；付费安装3旧合法候选 `[279,276,279]B` 和固定维护root `88af07516c0e101b260b7950f0e1de5afbfeb5e556952b4bd26b26a2be306815`；不改变旧三个候选/哈希和A预算，不直接改`version_ref_index`。凡研究者发起的动作，必须明确不是VM自主选择。

1. **LEGACY_WRITE**：索引前研究者有偿Write完整64B的目标hash到普通自有key→内核有偿Index记录引用→仍用原`Action::Write` CAS将旧引用改1B值；旧短写保持原referenced超集，`ReclaimIndexedVersion`真正Conflict。再真实重付完整Index后才Ok删除279B。保留**历史行为/安全基线**。
2. **DELTA_SINGLE**：与LEGACY同等完整世界snapshot与真实旧引用，但研究者采用新`WriteIndexed`合法CAS写成同一1B，须真正付费而且证书仍在、`referenced`中目标hash被安全移除，**不需第二次全索引**、真实GC成功释放279B；报告两臂合计A、索引A、证书序列化字节，区别于全任务A净收益。
3. **DELTA_MULTI**：两个**不同**真实普通对象均引用同一个目标原始hash，真实Index后首次`WriteIndexed`只改第一个→目标仍referenced/GC Conflict；再CAS改第二个→索引可证明无活引用/GC Ok。不能以任何“首次移除即可删”的错误引用计数逻辑通过。
4. **DELTA_ADD_AND_STALE**：最初无引用成功Index；新`WriteIndexed`把64B真hash写入普通`/self/pins/`，必须及时计入referenced/GC Conflict；用过期None CAS尝试覆盖必须Conflict且0 A/内容、version、索引、storage均不变；用当前真版本CAS写1B之后才可安全GC Ok。不能在新动作中只处理“引用删除”而忽视引入。
5. **VM_DELTA**：研究者有偿一次安装合法VM根，根使用新`WorldCall::WriteIndexed`在**活动VM真实tick**中CAS移除一个之前存在的hash pin；索引预先真建且根已合法激活，审计必须看到`Action::WriteIndexed`来自实际`tick_autonomous`；VM受限program返回且后来真正GC删除目标279B，B20未重复，无任意oracle。
6. **TRANSPORT_LEASE**：独立小型真实`World::new`世界（`message_delay=32`，无假网络服务），使用实际program安装、真实Send完整64B目标hash到C等待交付，随后付费Index；即使`WriteIndexed`删除本地唯一hash pin，来自真正`pending_deliveries`的旧source仍须阻止快速GC。此守卫不能因为只扫自有对象就忽略传输中的租约；此arm不是P2+B20对照。

每组真实audit核对Index次数/费用、WriteIndexed收费、GC结果、对象版本/存储和独立同一snapshot恢复后状态hash，长臂B恒20；拒绝不收费。出现原生红例保留运行链接，不改研究seed、既有验证器限制、原工具和安全判据。专项workflow仅针对新独立测试首次受控push，完成后恢复`workflow_dispatch`。

## 解释上限

该优化是**研究者先提供的通用世界内核/VM新能力**，不是生命体自己发明引用算法、主动维护候选、动态维护目标或一般全图安全；隐含认证假设仍仅“已安装版本的原始64B小写hash引用”，无法支持指针压缩、外部进程持有、编码哈希、新网络服务的生命周期。这个开发研究不进入正式E001确认性样本；**G0 GO / G1、G2 NO-GO，E001尚未校准/预注册**。


## 2026-11-23 — 原生首次六臂 6/6 全部通过

**[GitHub Actions #45163504990](https://github.com/iFrankStudio/OCEE/actions/runs/45163504990) @ `7e15bf9edc8b16706e561bc10151e9c2a8f966c5`**：6个独立Ubuntu Rust runner jobs均完成`success`，每组1 passed / 0 failed，本次**没有编译或机制红例**。以冻结在协议中的S29 world129/P2 seed42、固定三候选和真实历史B20为基础，新增的`WriteIndexed`及VM类型签名/桥接真正执行。原`Action::Write`和之前的S29维护root未改；逐对象source和transport source由内核从收费完整索引扫描生成，旧证书无source明细时新动作保守Conflict，不开放actor随意提供source计数。世界根内容hash（五个长臂中的四个研究者执行对照）仍为`88af07516c0e101b260b7950f0e1de5afbfeb5e556952b4bd26b26a2be306815`。

| 原生arm | 完整Index次数 | 初次Index A | indexed/普通Write A | 本臂世界Action A增量（包括pin、索引、回收） | 证书JSON大小（初次→末次） | 结果 |
|---|---:|---:|---|---:|---:|---|
| LEGACY_WRITE | 2 | 1668 | 原普通CAS `[4]` | **3353** | 1772→1606 B | 旧短写后仍保守Conflict，付费重建再真GC释放279B |
| DELTA_SINGLE | **1** | 1668 | 新差分CAS `[10]` | **1692** | 1772→1606 B | 同1B替换后立即安全解除唯一引用，真GC279B |
| DELTA_MULTI | **1** | 1670 | `[11,10]` | 1711 | 1881→1606 B | 两对象同一hash；仅删一个仍Conflict，全部删才GC279B |
| DELTA_ADD_AND_STALE | **1** | 1667 | `[15,0,10]` | 1700 | 1606→1606 B | 真正引入64B引用后Conflict；过期CAS真Conflict/0A，之后正确CAS才能删279B |
| VM_DELTA | **1** | 1655 | 活动VM实际WriteIndexed**10 A**，VM切片7 A | 不与上述研究者动作等起点比较 | 根不同 | 同一真实VM单次动作解除pin，实际GC279B，B20 |
| TRANSPORT_LEASE | **1** | 12 | 新差分CAS**5 A** | 独立无P2小世界，B0 | — | 真实Send在途原始64B hash，即使本地unpin仍真实GC Conflict、不误删 |

`LEGACY_WRITE`、`DELTA_SINGLE`、`DELTA_MULTI`、`DELTA_ADD_AND_STALE`的`VM A增量=0`是**诚实的实验界定**：它们确实在已执行完P2真实业务的同一世界中，由**研究者使用合法受费Action**测试内核状态转换，而非VM自己选择的GC策略。真正的VM调用通道由单独`VM_DELTA`认证，活动root `7793c7683dff968de818f1b4d8350d5f2b59cf76745d5175ae776379d50cf20a`经真实安装和CAS后，有`WorldCall::WriteIndexed`→世界实际`Action::WriteIndexed`，A动作10+VM7收费，个体收到真实Ok回执后返回U32(1)，并且原S29/P2 B20不重复。

### 因果对照与成本账本

`LEGACY_WRITE`与`DELTA_SINGLE`使用同一P2 world/修复工具、同一目标279B旧程序、同一pin、同一证书初始JSON大小1772B和相同第一次完整索引**1668 A**。区别只在删掉唯一旧引用时，旧普通`Write`写1B收费**4A**，保留旧证书过时的正引用，首次GC Conflict，然后**必须第二次全扫描**才删除279B；新原子`WriteIndexed`将1B写入收费**10A**，同一tick将该来源引用集合替换并按所有其它来源/在途消息重新计算认证并集，GC直接Ok而且无需重建索引。两条研究者动作区间的全部世界Action A **3353 vs 1692**，本开发对照中节省**1661 A（约49.5%）**；本质是避免一笔约1667A的全扫描，扣除新写入较旧写增加的6A。相同storage delta均 -278B（1B新对象 +279B回收）。不能把局部A差值推广为全任务、长期净资源收益，更不能把B与A当同一单位相减。

### 严格安全边界

- `DELTA_MULTI`在唯一一次全扫描之后，第一次删除一个对象中的引用仍真GC Conflict，不会误删；第二个不同key合法更新后才放行。证明没有把单个引用消失当成全局无引用。
- `DELTA_ADD_AND_STALE`真正新增64B原始hash引用必须立刻出现在证书中，GC先Conflict；过期CAS 0A必须不碰对象版本、数据、storage、认证索引，只有真实当前版本CAS后才放行。
- `TRANSPORT_LEASE`独立世界`message_delay32`中先真实Send到C，再付费Index，真实pending消息hash被认证为不可更新的transport source；合法`WriteIndexed`移走本地hash仍不移除传输层引用，GC继续Conflict。此项**没有真实外网，仅确定性内核消息队列**。
- `VM_DELTA`证明**新能力可以由真正活动VM请求**，但程序、根切换和待删旧hash仍由研究者提供；内核仍需真正执行一致性校验。没有让VM自发发现候选或主动开始维护。
- 源集合存于kernel的`VersionRefIndex`内存和快照；实测第一次证书JSON `1606–1881B`（取决于已知对象和引用），存在额外索引元数据占用。新增元数据不计入个体storage_used，建索引沿用原完整扫描费，新动作显性为它付费复核；**这还不是完整的索引内存经济学**，不得免费忽略占用。
- 老`Write`短写及其它潜在变更来源仍全量保守失效，旧证书可能遗留过保守引用；当一份程序版本真GC成功时，旧per-source成员也可能保守保留到下一次完整扫描，不能误称完全精确引用追踪或通用编码指针/异步安全。

**研究闸门维持G0 GO / G1、G2 NO-GO，正式E001尚未校准/预注册。** 本次6/6是**专项原生**验收，不冒充整个Rust workspace的新全量CI结果。后续应优先测量多轮真实混合更新下索引大小/维护费用随对象数和引用数增长的边界，以及一次Program引用可被旧注册/不透明结构保留的安全边界；动态候选发现/目标内生生成仍独立待研究。专项workflow在归档后恢复为`workflow_dispatch` only。
