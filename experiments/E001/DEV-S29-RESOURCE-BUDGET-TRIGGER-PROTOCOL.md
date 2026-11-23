# DEV — S29 有偿资源观测驱动维护时机，拒绝低收益/低算力的昂贵全量扫描（实施前冻结）

**状态：先冻结开发对照；首轮4/5暴露夹具非穷尽，先登记失败与仅夹具修订，随后原生完整5/5通过并归档。** 基线 `68e8fe661d5e50f89acc5e7bdca1d3c7c1451dd4`。此前[受费目录发现](./DEV-S29-PAID-OWN-INVENTORY-BOUNDED-DISCOVERY-PROTOCOL.md)原生4/4验证真实VM从自身版本目录中取得动态程序哈希并真GC，但在无论是否缺空间的情况下都支付目录27A、完整索引约1.6k A与大量VM指令；一轮典型维护VM700+Action1702=2402 A只回收276B。现需研究有限个体能否基于自己真正感知到的资源存量决定**不启动**昂贵的维护，而非盲目尝试。该实现只修改研究者提供的VM策略/测试，**不增加内核读取能力、VM指令或GC权力**。

## 明确研究前的固定策略（不从结果调参）

- 与上轮同一个世界内核通用受费 `Read("/sys/resource-status")`：真实返回JSON `{"used":U32,"capacity":U32,"a_available":U32}`，行动层收费2 A，值在支付后采样，返回的A是剩余gas。环境数值不适配U32时`Quota`，不允许隐式截断。
- 由研究者构建一个**有限、可静态验证、内容寻址的自有VM程序**：真正先`WorldCall::Read`自身resource-status，按真实`ActionResult`检查`Ok`，从其`payload`的JSON提取`used/capacity/a_available`。只通过现有类型化通用`Sub`、`Div`、`EqU32`计算门控：
  - `free=capacity-used`，前置合法世界保证capacity>=used；`free/4096==0`表示**小于4KiB剩余**，才考虑维护；>=4KiB绝不查询目录/索引/GC。
  - `a_available/3000!=0`表示仍**至少3000 A当前epoch算力**，才开始一次可能消耗约2402 A的有界维护；<3000 A即便空间不足也主动停止，避免新动作无望地扩大开销（不能因此承诺不会耗尽gas；真实路径仍完全受内核A预算约束）。
  - 当两项门槛都满足，跳进**上一轮完全相同的目录第0页读取→付费Index→RepeatBounded(8)动态GC/真Conflict反馈**；第一个成功退出、没有则返回0；没有候选hash字面常量。闲置/资源不足返回U32(0)，执行成功返回1；两个返回值并未区分“无需维护”和“尝试后失败”，本轮仅以audit确证行为，不能混称内部风险记忆。
- 4096 B及3000 A两个阈值完全是**研究者固定的先验启发式**，不是个体自主学习、最优效用、A和存储的可换算价格或“自主创造目标”。它明确牺牲有空闲空间时的早期清理机会，换取避免高成本扫描；存在低空间却所有候选被引用时仍会浪费费用的**反例**，必须如实记录。
- A与存储B仍是不同资源：不得把`A`和回收`B`直接算成净能量收益；必须分别记录真实world Action A、VM A、目录与索引收费、回收B、原P2 B20、节点capacity/used/balance、audit/status结果和快照确定性。

## 先冻结的5臂原生Rust对照

共用**同一研究者提供的门控VM字节码、根hash**与相同启动前历史P2 world129/protocol42正确B20、A gas_per_epoch=12000。先付费真正安装三候选总834B（目标hash仅在测试判据中用于核实，不在VM字节码）；研究者一次受费active-version读取+CAS激活VM；同一次研究者提供的观察能力`/sys/resource-status`，不读GC安全标签。两组容量差异用研究者明确调整`node A.storage_capacity`，属于受控世界配置，**不是个体行动**。

1. `PRESSURE_EXECUTE`：capacity=used+2048（空闲2KiB）、正常A预算至少3000；活动VM真Read资源2A后必须真实Read目录（上轮27A附近）、真Index一次、真正动态GC按Conflict跳过活root/registered并至少成功释放一个原合法候选，所有目标在实际目录；终态VM返回1，原业务B20，快照恢复一致。记录完整维护增量A及回收B，不要求其比其他已冻结不同root的总A更小。
2. `SPARE_SKIP`：除capacity=used+8192（空闲8KiB）之外与PRESSURE初态对象、根、A预算相同。VM仍必须真Read资源2A（收费）、**零次**目录/Index/GC，返回0，所有3旧候选保持、used不变；报告与压力世界的付费维护差额。这个节约是“本次不清理”的**算力节约**，绝不能声称回收能力提升。
3. `LOW_A_SKIP`：真实研究者定时`ScheduledEvent::ResourceChange`把A节点下一epoch的`gas_per_epoch`改为**2400 A**，等待真实epoch发放之后才启动同一根，容量仍空闲2KiB。真实Read资源2A之后`a_available<3000`，故零次目录/Index/GC，返回0、未删除任何候选。使用实际确定性资源变化事件而非直接覆盖gas余额；记录epoch和事件。B20保持；快照一致。
4. `RACE_AFTER_SENSOR`：空闲2KiB，活动VM真正付费读资源确认满足门槛后暂停于其原有continuation；**研究者付费写入64B原始哈希的独立pin**，然后继续同一个VM；必须最终目录、Index读取现实变化并真实`Conflict`拒删被pin对象；其它可删候选可成功，不得绕过引用证书/省略检测。记录status到GC之间的step、实际费用、快照重放。此臂不声称策略具有并发预见性。
5. `PROTECTED_ONLY`：空闲2KiB且正常A；研究者在启动前用真实收费`Write`将全部3个本来可回收的版本哈希分别写入独立自有pin路径；活动VM仍根据低空闲/高A决定执行目录+付费Index，逐项对所有活引用真实Conflict，返回0、回收0B。必须展示**条件门控在高压力但无实际可回收资产时仍然浪费维护费用**。不能把这臂删去，也不能为此降低阈值或作弊植入安全oracle。

录入的`VersionRefIndex`仍是真实按原收费扫描生成；所有GC保护和老S29 P2正确业务无改动；支持从同一snapshot分叉比较确定性，不重启活动根、不切换至第二策略。测试路径定向5独立native jobs，先冻文档后写代码再一次受控CI，失败保留原run并只修代码/fixture而非判据。成功后恢复专项workflow为`workflow_dispatch` only。

**研究位置：G0 GO、G1/G2 NO-GO，正式E001仍未校准/预注册。** 本轮只检验研究者给定门槛策略能否做真实付费跳过，不表明自发创造生命目标、持久策略优化或开放式演化。


## 首轮红例与仅夹具修订（原验收判据不变）

**原生[run #45202464053](https://github.com/iFrankStudio/OCEE/actions/runs/45202464053) @2af48822c8aabdf21286dd6672d5165e678318e6：4/5，`PROTECTED_ONLY`失败，另4臂成功。** 失败为`Some(U32(1))` vs 预设`Some(U32(0))`，发生在[本轮Rust测试](../../crates/organism-seeds/tests/s29_reclaim_version.rs)的受保护负例断言。根源：此前仅对**三个人工追加候选**创建有偿原始64B pin，但前置P2自有版本目录不只这三个可用目标；同一真VM还能合法发现另一个未pin候选并成功回收，因此该fixture并未满足先冻协议的“无实际可回收资产”前提。不能把这个结果解释为安全回收错误或删除测试判据。

**在修代码之前冻结的修正**：改为在`PROTECTED_ONLY`启动前，研究者遍历真实世界当前所有actor拥有的 `/self/versions/<64hex>` 实际版本，**每一项都通过真实收费`Action::Write`将64B原始hash写入独立普通`/self/pins/s29-gate-<n>`对象**，仍包括原预设3个候选，但也覆盖P2历史安装的其它版本、活动根及不可回收根。该夹具更严格，保证整页所有8条都有可验证的独立活pin，而不依赖作者主观认定“只三条可删”；要求`used+pin_bytes<=capacity=used+2048`，真实资源sensor将读取实际减少的空闲字节数，仍属于`<4096B`高压力状态，正常A大于3000。此修正只改变研究者建立“确实没有可回收对象”反例世界所需的真实对象/费用，不修改VM逻辑、任何已有arm、阈值4096B/3000A、P2 seed/任务或该反例的**零GC成功、U32(0)、有偿目录与索引、完全snapshot replay**预设判据。红run及其根因永久保留，修正后新增CI run单独登记。


## 2026-11-23 — 首轮4/5与修正后完整5/5的原生证据

- 第一轮 **[Actions #45202464053](https://github.com/iFrankStudio/OCEE/actions/runs/45202464053) @`2af48822c8aabdf21286dd6672d5165e678318e6`：completed/failure，4/5**；`PROTECTED_ONLY`在仅pin3个研究者追加版本的夹具中意外真实回收另一个既存版本并返回1，严格不是GC越权，而是负对照**缺乏“所有版本受引用”的事实前提**。完整日志与冻结后的修正见上面的“首轮红例”章节，未隐藏、未删除、未降低判据。
- 第二轮 **[Actions #45203295504](https://github.com/iFrankStudio/OCEE/actions/runs/45203295504) @`8560eb39adacf1201685cacdee1b2661cfdff2bb`：completed/success，5个独立Ubuntu Rust jobs全部1 passed/0 failed，5/5**。修改仅`PROTECTED_ONLY`测试夹具：研究者对当前世界所有10条真实安装版本使用真受费普通Write建立一对一64B活引用（合计新增640B物理数据，而不是只给额外3条pin），真实索引扫描会发现全部活引用。**资源门槛、VM字节码、其它4臂、P2世界seed129/protocol42、A12000、动态catalog/GC规则和目标判据均未改**。同一研究者装载的有界维护根hash `534ab1585013d573b310fa3aa5b8aa462d6e18c486c8be55047167bfee4bc2c8` 在5臂完全相同，真实维护根不包含任何3个旧候选的完整64B原始hash字面值。

| 修正后arm | 真实Paid状态观测 | 目录读取/付费Index/GC | VM A | 世界Action A | 维护A合计 | 真实回收 |
|---|---|---|---:|---:|---:|---:|
| `PRESSURE_EXECUTE` | 余量**2048B**；当前A **11157** | 1次目录，1次Index；`Conflict, Conflict, Ok(8A)` | 780 | 1704 | **2484 A** | **276 B** |
| `SPARE_SKIP` | 余量**8192B**；A **11157** | **0目录/0Index/0GC** | 82 | 2 | **84 A** | 0 B |
| `LOW_A_SKIP` | 余量**2048B**；下一真实epoch资源事件让A每epoch=2400；传感器读后当前A **2394** | **0目录/0Index/0GC** | 81 | 2 | **83 A** | 0 B |
| `RACE_AFTER_SENSOR` | 先真观测余量2048B/A11157，再研究者真实受费写64B活pin | 1目录、1Index；`Conflict,Conflict,Conflict,Ok(8A)`；新pin未误删 | 868 | 1711（**含研究者6A的pin**） | **2579 A** | **279 B** |
| `PROTECTED_ONLY` | 研究者真实付费预先pin**所有10项**，空闲1408B、A11097 | 1目录、1Index；前8项**全部Conflict/0A** | 1236 | 1709 | **2945 A** | **0 B** |

从本轮`start`到VM结束的`VM A`与`Action A`都是真实`World.audit`相加，其中所有成功/失败Action与VM切片都有原生记录；表内**不包括研究者此前安装维护根、P2业务修复、PROTECTED_ONLY用于配置环境的10次pin的A费**，这些初始环境变更本身仍有审计并非“生命体免费资产”。`LOW_A_SKIP`的跨epoch空tick与受控`ScheduledEvent::ResourceChange`也发生在维护`start`之前；并非说资源事件是VM创造的。全5臂初始P2正确业务均B20，没有重复执行；根只被研究者激活一次、VM后未主动切换，所有最终snapshot经Rust `World::from_snapshot`恢复后完整重放一致。

### 能成立的配对结论与必须保留的负例

- **仅改容量的同根双臂：PRESSURE vs SPARE**，同一个根、同样10项真实版本与正常A余额，同等初始owned存储used=110194B，唯一受控配置差异是A存储总capacity（used+2048B vs used+8192B）。每臂先实付2A资源状态读取，饱和时允许下一步真目录/Index/GC，在空闲时主动返回0且**后续没有任何目录/Index/GC**。两世界维护区间付费 **2484A vs 84A**，空闲状态对照**避免2400A**开销。它也放弃了本可回收的276B旧版本，并非“资源收益无成本”。
- **LOW_A_SKIP**通过真实调度环境事件在下一epoch将A预算变为2400，真实传感器读后余2394A，虽然只剩2048B，也自愿阻止目录/Index/GC。证明可以对实际稀缺算力实行简单止损，但未证明3000A总能保证一轮维护成功。
- **RACE_AFTER_SENSOR**说明当资源状态读取之后新增真hash引用，VM会再经真实目录与引用Index确认，后续对被pin的动态目标收到真正`Conflict`，不会因旧的资源状态而越过GC安全边界；代价是本轮更新后它仍然支付扫描与循环开销。
- **最有价值的负例PROTECTED_ONLY**：10个版本均被研究者真受费长引用保护后，尽管资源传感器真显示1408B严重短缺、A余额充足，VM仍执行完整有偿目录、Index、8次GC且全被真实`Conflict`拒绝，**维护耗费2945A、释放0B**（前置构造活pin的研究者付费未算入）。这否定了“仅看空闲空间与A余额就能判断维护有收益”的强主张。后续必须引入不泄漏安全oracle的可验证回收机会/预估收益和更严格准入机制，而不是只微调4096/3000两个作者阈值。

### 科学边界与下一步

当前进展是**研究者构造的受限VM策略真读取资源信息并在多种世界配置中作不同决定**，并不是自主生成维护目标、学习合理的A/B交换率、或内生策略演化。策略不会自己在后来资源变化时重启/苏醒，只有研究者初次启用并在单次continuation内运行；无长期福利或多seed统计。受费目录、引用Index、GC的原始64B ASCII哈希契约没变，外部服务和编码引用未证明。研究下一突破前必须处理“全部受保护但空间严重紧张时的零收益高成本”问题，可能研究基于**真实付费引用证书的受限机会估算和成本上限**，但不能赠送隐藏的可回收列表或把研究者预选遗弃版本换个名字。

**G0 GO、G1/G2 NO-GO；正式E001未校准/预注册。** 本轮只验收5个定向Rust jobs，不冒充全工作区回归。专项CI归档后恢复`workflow_dispatch` only。
