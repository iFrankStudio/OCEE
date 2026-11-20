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

## 2026-11-20 原生验收：消息发送与服务消费两阶段真实竞态

- [**定向原生 S39 CI #44704534899**](https://github.com/iFrankStudio/OCEE/actions/runs/44704534899) 已 **completed/success，4 passed / 0 failed / 104.06 秒**；验收 Rust 代码 SHA [`2f9a0b7`](https://github.com/iFrankStudio/OCEE/commit/2f9a0b71bddbb559714763790005255f2a15e58b)。正常继承、v2合法再学习、Send前TOCTOU、Send后C处理前TOCTOU、内核凭证伪造与授权失败等均由真实tick执行。同期 [S38 回归 #44704534991](https://github.com/iFrankStudio/OCEE/actions/runs/44704534991) 也 completed/success，但全 workspace [#44704534900](https://github.com/iFrankStudio/OCEE/actions/runs/44704534900) 尚需单独核验。
- **共同冻结根和正常 P1→P2**：S39 初始 root hash `23c42373289b15c801a9b48d5c2599d478d6da447ac9750c0d1d1de8410c651e` 在 P1 seed1 / P2 seed55 完全相同。P1 真正生成组合程序、真实经两次付费 WRITE 导入 P2（import29 A）。两世界独立付费 Probe/缓存校验/生成真实 B 第四数，VM 成功发出一次内核 `ArmTaskFence`（3 A）后，协议专家 Send、C 队列服务消费全部合法，实际 B gas **每组24**。P1 A **15,237**、P2 A **26,150**；成功消费的受保护消息记录被正常清理，`fence_drops=空`，世界快照一致。
- **正常v1→v2重新学习仍可用**：P2 seed55 在首次付费查询/持久缓存后遭研究者许可的 v2任务规则变化，按S38真实路径续期7 A、第二次主动Probe52 A、证据CAS v0→v1、原始回执再次核验，接着活动S39根对**新任务v2的回执**Arm新指纹栅栏，真实算出 v2 第四数 **10**，C/B实际执行成功 **A26,794 / 262 ticks / B24**；守护机制并未误阻断合法升级。
- **Send前任务漂移**：S39世界先实际执行到 `ArmTaskFence` Ok，而S38对照先真正创建同一 v1旧操作数；随后研究者变更任务v2。S39之后真实 `Action::Send` 被内核在入队和资源扣费之前返回 **Conflict**，没有成功发送、没有新增队列项，**B0**；S38无栅栏对照则成功发送，真实B gas **24**，第四操作数旧值 **48** 而v2真实目标为 **10**。
- **SEND之后、C/B消费前漂移**：原生测试分别观察到两个世界真实 `Action::Send`，其内容可由当前协议解析为 **REMOTE_EVAL，4个U32，18字节body**；两组实际消息id均为 **26**，已进入 `pending_deliveries`，此时还未发生与该请求相关的B gas。研究者随后修改公开任务v2并改变环境真实oracle；S39世界内核在 `process_service_inbox` 检查该 id 保存的任务hash 与当前hash不同，直接丢弃请求并将 `TaskFenceDrop{message_id:26,expected_hash,observed_hash,step}` 留在世界状态，不将消息交给B协议服务，不发生B计费，**B0**。无栅栏 S38 对照则消耗 **B24** 并计算旧操作数 **48**（v2正确 **10**），证明仅发送时验证仍不足以避免在途竞态。
- **直接内核动作验证**：没有任何原始签发的 world 提交假回执请求Arm，返回 NotFound并收费3 A；合法Probe之后用伪造回执Arm返回Conflict并收费3 A；持真实原回执且任务未更改时可Arm成功；随后任务改变，原回执不再可用于更新栅栏，显式直接构造的Send仍Conflict，**不消耗发送费、不消耗C带宽、不产生排队消息**。世界快照→还原与状态哈希一致。

### 科学边界与后续缺口
- 此测试只覆盖**明确由当前个体先用真实回执建栅栏**的消息。没有栅栏的旧程序和建栅栏前已经入队的消息，不会被自动追溯保护；因此它不等于让任何数字服务默认具备任务一致性。
- 这里的第二次检查位于**同一个确定性世界内核内**，在真正服务消费及B资源扣费之前，是可信模拟内核的串行原子边界；不等于现实互联网里的跨进程分布式事务。`process_service_inbox` 处理完成后再修改公共任务不能自动回滚已经完成的B输出。还没有覆盖恶意内核、消息指纹被改写、网络分区、重试/超时造成的复杂幂等性、在服务内部执行期间的并发变更、用户侧实际外部副作用。
- 专家、两输入的主动Probe银行、任务v1/v2、候选54图、一次续期和内核栅栏均为研究者先验。S39仅是对**特定TOCTOU故障**的原生因果实验，不能被用作自主演化、通用环境适应、真实网络自动运行或通过E001 H3的证据。**G0 GO；G1/G2 NO-GO，E001未预注册。** 下一值得优先探索的 S40 是受保护消息的**失败后恢复和幂等重试**：识别C拒绝原因、在版本更新后再学/再发并确保不会重复B计费；仍必须坚持任务级资源账本与因果消融。