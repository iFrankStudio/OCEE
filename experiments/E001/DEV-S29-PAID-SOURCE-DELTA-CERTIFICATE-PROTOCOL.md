# DEV — S29 内核认证逐来源引用差分、真CAS写入与旧证书保守对照（执行前冻结）

**状态：研究协议冻结于实现前，原生结果尚未验收。** 基线 `65bdff70f0c7762f82bf444ab54af64dde11e84d`。上一[严格短写入认证保留](./DEV-S29-CERTIFIED-SHORT-WRITE-PRESERVATION.md)原生4/4证明1B短写可以安全保留原有证书，却在一条旧64B哈希引用被覆盖时仍必须保守拒绝回收，只有第二次真正付费全索引才能解除旧标记；这是**假阳性旧引用**，不是新的活引用安全证明。本轮只改进现有64字节原始ASCII哈希模型，不宣称任意编码/间接引用、分布式租约或开放式演化。

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
