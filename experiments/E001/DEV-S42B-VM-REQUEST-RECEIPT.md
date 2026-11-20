# OCEE S42B — VM 原生持有完整 u64 请求回执与指定请求故障归因（待验证）

**日期：2026-11-21。工程候选已提交，GitHub-hosted runner 尚未恢复。本记录仅声明静态实现/可证伪测试，不声称原生运行通过。**

## 假说和与 S42A 的区别
S42A 内核以 `last_guarded_send[actor]` 猜测当前需恢复的消息，仍可能在多个受保护 SEND 中误判等待对象。S42B 的活动 P2 个体不读取宿主日志或私有 `last_guarded_send`：它接收实际 `WorldCall::Send` 的 `ActionResult.payload`（8字节、LE u64），仅在真实 B 计算请求对应的 phase=1、SEND成功、长度为8时，`WorldCall::Write` 将这个回执写到自己的 `/self/work/s42b-remote-request-id`。

当真实 `Receive` 返回 `Empty` 时，活动 VM 仅在 phase=1 付费读回自己的 ID（必须为8字节），执行 `WorldCall::FenceStatusFor(id)`。仅在内核确认该请求真的被丢弃后，才使用**相同 ID**调用 `WorldCall::RestartRootFor(id)`。S42B 的128次循环到期直接失败，不能回退使用 S40/S41 无 ID 的角色级状态检查。新程序是独立版本，原 S40/S41 程序源码不修改；重启会依S41生命周期合同删除工作区旧回执，新尝试再次真实取新的 SEND ID。

## 内核安全边界
新动作 `FenceStatusFor{actor,request_id:u64}` 和 `RestartRootFor{actor,request_id:u64}` 不信任普通 `/self` 对象中的数值，只将其作为**查询索引**：实际资格来自内核不可由个体写入的 TaskFenceDrop、当前公开任务的非空 hash、记录的 actor/id 及 observed≠expected。若同一 actor 的**其他受保护请求仍在途**则拒绝重启；只要单个当前模拟世界已经有 B gas>0 的服务事件，也保守拒绝重复恢复。现阶段是一世界单个体、可信本地服务策略，可能错误阻止并发任务恢复，但不能据此声称真实网络 exactly-once。

ABI 坚持 `request_id` 为**8字节 LE u64**，不经 `u32` 截断；错误长度通过 VM 类型/宿主动作转换 `BadRequest` 拒绝。两种 ID 动作复用 S41 的状态询问1 A以及 S40 的最多一次重启8 A，不放宽原有程序版本/CAS/权限。

## 两项最关键测试（提交但未运行）
1. **内核并发反例**：实际SEND产生两个大于 `u32::MAX` 的消息ID，旧请求丢弃而另一条仍在途时，按旧ID查故障必须 `Empty`，请求重启必须 `Denied`；两条都真实丢弃后按分别的正确ID可获得 `Ok`，截断ID/伪造ID仍 `Empty`，B0、世界快照可重放。
2. **同一活动 P2 VM 因果链**：root由研究者预置，P2专家实际发送四操作数 B 请求并从反馈保存超32位的ID；任务v1→v2时原请求真实TaskFenceDrop，随后审计必须见到 VM 发出的 `FenceStatusFor(old_id)`、`RestartRootFor(old_id)`（不能出现S40/S41旧式角色级调用），重启/重新Probe/修复/CAS后第二个新请求真实计算B24且旧B0、B恰好一次，快照一致。

## 研究边界与后续
任务、P1/P2协议专家、研究者给出的phase=1规则、有限重试、内核强一致服务、S41三种清理前缀、54种候选仍为人工先验。单个VM保存8字节消息ID不是任意分布式消息归因或自主形成生命。下一步需要消息处理成功/无响应/服务后故障的三值状态，以及 ACK/RESULT 与请求ID的服务端回显绑定；同时逐步将对象生命周期从路径分类改为来源依赖。

**闸门**：S42B待CI，S41/S42A未原生验收。G0 GO、G1/G2 NO-GO；E001未预注册。恢复 runner 后 S42B窄测试→S42A内核→S41/S40定向→必要时workspace；保存包含失败的完整证据。

[VM个体](../../crates/organism-seeds/src/s42b_request_receipt.rs) · [内核动作](../../crates/world-kernel/src/lib.rs) · [世界调用ABI](../../crates/world-kernel/src/program_host.rs) · [组合测试](../../crates/organism-seeds/tests/s41_early_drop.rs)
