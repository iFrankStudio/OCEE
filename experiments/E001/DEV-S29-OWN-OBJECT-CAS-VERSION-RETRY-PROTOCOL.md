# DEV — 自身对象原子内容+版本读取与多次GC失败记忆CAS（先冻结）

**状态：实验协议先于实现与原生CI冻结；暂无本轮结果。** 基线 `b243990f60bf5d44432d434895bd47cae6826b9d`。紧接[同根单次激活三轮retry/持久Continuation](./DEV-S29-BOUNDED-SELF-RETRY-CONTINUATION-PROTOCOL.md) native3/3：原世界`Read("/self/maintenance/risk-s29")`只有原始正文，不返回`Object.version`，上轮VM写b"1"时预置CAS None只在首次缺失成立；第三轮再次真实GC Conflict后对象已v1，记忆写Conflict而无法延续动态历史。不能将失败CAS当作成功学习；需要真实有偿观察版本，不能无偿偷读内核版本。

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