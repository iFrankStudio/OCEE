# OCEE S39 — 任务版本栅栏：真实 Send 与 C/B 消费双重检查

**2026-11-20；开发性因果实验，非E001预注册确认。G0 GO；G1/G2 NO-GO。**

## 明确的竞态故障
S37 在每次接受缓存证据前向内核 VerifyProbe，S38 在环境任务从v1变v2时可获得一次有费续期、重新采样、CAS替换证据和修复程序，但两者在**最后一次验证之后、最终 C/B 真正处理远程请求之前**存在时间竞态。世界没有外部网络服务，这是确定性模拟世界里的真实消息队列与独立 B 计费：消息已入队也不能保证它在未来被服务消费时任务未变。

研究者明确将 v1 开发任务 `s36-interactive-probe-v1` 的隐藏规则 `hidden_compose(seed,input)+depth×Len(input)` 变为 v2 `s37-changed-rule-v2` 的 `2×Len(input)`，并更新公开两条标注和公开对象版本。这一任务修改只由实验宿主执行，个体并无公开写权限。对 P2 seed55 的未标注输入，v1和v2答案分别**48与10**；普通 FOLD32 B 仍可正常处理一个来自v1的错误操作数，因此必须同时检查任务语义和B服务实际是否消耗资源。

## 机制：活动个体证明任务证据，内核在两个原子边界强制执行
- **受费建栅栏**：活动 S39 VM 在已执行S38的双标签+有费环境Probe/证据Verify，且实际计算第四U32并写入 `REMOTE_VALUES` 后，立刻重新READ自有证据对象，向内核调用新增 `WorldCall::ArmTaskFence(evidence)` → `Action::ArmTaskFence`。只在当前task SHA256与内核保存原始签发回执的task_hash一致、且提交的原回执字节相等时，内核才将 `task_fences[actor]=task_hash`；收取3 A。该栅栏由运行中的VM主动建立，不能通过普通自有对象Write伪造；并不是第二次查询、新任务学习或隐式服务认证。
- **Send事务点**：一旦该 actor 建立栅栏，既有专家的每个真实 `Action::Send` 都必须在资源扣费/带宽扣费/消息入队之前验证当前公共任务字节哈希等于栅栏哈希；不匹配返回 `Code::Conflict`，不扣发送费/带宽、不新增排队消息。未建立栅栏的原S38旧控制保持原有发送机制，不受影响。成功入队的消息保存 `guarded_sends[message_id]=原任务哈希` 到世界状态；已有专家的格式/协议帧保持不变。
- **C/B执行事务点**：在 `process_service_inbox` 真正将 C 消息交给本地协议模拟器、扣除 B gas 前，对该消息的guarded_sends进行第二次当前任务哈希比较；发现环境在传输中漂移就直接丢弃，不调用service.handle、不扣B gas，将 `TaskFenceDrop{message_id,step,expected_hash,observed_hash}` 写进可持久化世界状态。对于带宽不足而延后处理的消息，保持其 guard；仅成功处理或确实被丢弃时才移除对应guard，避免把延期的消息变成无保护消息。
- `task_fences`/`guarded_sends`/`fence_drops` 都持久化到 snapshot/state hash，未使用时采用 serde 缺省并省略空集合，意在保持历史无栅栏世界的序列化格式。`ArmTaskFence` 也是受VM静态类型校验和真实tick审计的受限原语；它不允许写公共任务或开放网络访问。

## 预先指定的原生因果对照
- **正常P1→P2**：source P1 seed1 在任务v1真实学习并生成图；目标P2 seed55真实导入同一程序、独立采样与验证；VM都在第四B操作数已经写好、专家发送前主动Arm一次，发送/入队/实际C/B处理均通过同一版本栅栏，B各24，独立oracle与真实协议帧一致；栅栏消息处理后guard map为空，fence_drops为空。另需保留S38在目标规则合法变为v2时二次受费查询+CAS恢复后，S39改为对**v2新回执**建栅栏、正常B24。
- **变更在栅栏后、Send前**：将S39原生世界运行到其真正Arm成功，科研方仅改变公共任务及隐藏oracle为v2，不修改VM程序；目标后续真实Send应在入队前被内核Conflict拒绝，无B计费。对照的S38世界在旧操作数生成后同样更改任务，由未受保护专家继续真实Send/B24，其四号操作数不符合新oracle。必须检查真实Action审计和 pending_deliveries，不允许只相信预期分支。
- **变更在远程 B SEND之后、C处理前**：在两组世界中真正观察到 `Action::Send{bytes}` 可解码的 `RemoteEval`（数量4、payload18字节），提取实际消息id并确认尚在 pending_deliveries；研究方随后更新任务v2。S39成功Send但带版本的请求应在C/B实际消费时通过第二次哈希比对被丢弃，fence_drops明确记载相同消息id，service_events不得出现该消息的B gas；无栅栏S38对照的同一个合法请求继续执行B24，但数据第四操作数已经过期。
- **原语安全与成本**：未查询的world不能Arm（NotFound、3 A）；真实Probe之后提供错误回执不能Arm（Conflict、3 A）；原回执且同任务可Arm（Ok、3 A）；之后任务改变时旧回执不可再次Arm（Conflict），已建立的栅栏也不被偷偷修改；即使研究者直接构造 `Action::Send` 也无法绕过栅栏，Conflict时无A/带宽扣费、无入队消息；snapshot重载保持内核状态与审计。

## 可支持的结论及明确限制
如果原生结果通过，可声称**在这个单核、确定性模拟环境中**，任务哈希由活动VM提供真实探测凭据建立，内核在发送和B消费时进行双点强制校验，防止任务两次典型TOCTOU漂移导致错误B计算。**不能称为真实互联网的分布式原子提交**：服务外部副作用不受此内核控制，若已经完成B计费后再变更任务没有自动回滚；也不能阻止恶意内核、具备修改公共任务与内核记录权限的攻击者、同步交换未授权框架或任意多任务版本演化。原始根的18基础候选×3阶段、探针建议、升级协议、两位协议专家均为研究者先验。**G0 GO，G1/G2 NO-GO，E001未预注册**。

[S39活动VM建栅栏](../../crates/organism-seeds/src/s39_task_fenced_remote.rs) · [内核SEND/C处理双点检查](../../crates/world-kernel/src/lib.rs) · [4项原生因果测试](../../crates/organism-seeds/tests/s39_task_fence.rs) · [专用CI](../../.github/workflows/s39-task-fence.yml)。

**真实结果按CI日志追加。**