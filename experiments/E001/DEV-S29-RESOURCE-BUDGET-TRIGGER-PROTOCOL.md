# DEV — S29 有偿资源观测驱动维护时机，拒绝低收益/低算力的昂贵全量扫描（实施前冻结）

**状态：预注册开发对照、原生结果未验证。** 基线 `68e8fe661d5e50f89acc5e7bdca1d3c7c1451dd4`。此前[受费目录发现](./DEV-S29-PAID-OWN-INVENTORY-BOUNDED-DISCOVERY-PROTOCOL.md)原生4/4验证真实VM从自身版本目录中取得动态程序哈希并真GC，但在无论是否缺空间的情况下都支付目录27A、完整索引约1.6k A与大量VM指令；一轮典型维护VM700+Action1702=2402 A只回收276B。现需研究有限个体能否基于自己真正感知到的资源存量决定**不启动**昂贵的维护，而非盲目尝试。该实现只修改研究者提供的VM策略/测试，**不增加内核读取能力、VM指令或GC权力**。

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
