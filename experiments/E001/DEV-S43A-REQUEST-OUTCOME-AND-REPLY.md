# OCEE S43A — 远程请求结果状态与真实回复因果验收（待原生验证）

**2026-11-21，工程候选。源码/定向因果测试提交；GitHub Actions 运行器尚未证实恢复，本环境无 cargo/rustc。以下测试条件不是已获得的运行结果。**

## 问题
S42B 的个体能保存实际 SEND 回执、按 u64 ID 查询丢弃并请求一次恢复；但 RECEIVE 无回复时不能据此推出「B 没执行」。服务可能已经消耗 B、只是在等待 ACK 传输。协议的 `request_id:u32` 是服务逻辑工作 ID，**不是** 内核 `Message.id:u64`；两者不能混同。即便模拟协议返回了算术成功，也不能证明该回复属于个体当前等待的具体 SEND。

## S43A 最小机制
- 新受费 `RequestOutcomeFor(request_id:8字节LE u64)`，内核首先查验最近4096条审计中该 actor 曾真实 SEND 成功并获得该 ID，再从 `pending_deliveries`、C 邮箱、`TaskFenceDrop` 和实际 `ServiceEvent` 得出 `Unknown` / `InFlight` / `Dropped` / `HandledNoB` / `CommittedB`。一律收2 A gas，未知**不是失败证明**。服务事件以真实 B 扣费为 `CommittedB` 基础，不表示工作结果已经被客户收到。
- `CommittedB` / `HandledNoB` 同时返回实际 `response_message_id:u64` 和可观测的队列位置：`InTransit`、`Mailbox` 或 `NotQueued`。后者**不代表丢失**，也可能已经被 RECEIVE 取走。
- 新受费 `ConfirmResponseFor(request_id:8字节LE, reply:实际Message JSON)` 必须同时满足：同 actor 的实际SEND回执、`ServiceEvent.request_message_id→response_message_id` 真实映射、响应来自C且发送到A，并且最近4096条审计中 **真实成功的 Action::Receive** 确实交付过完整且逐字节相同的 `Message`。未收到时仅猜测服务器回复或篡改返回字节都必须拒绝。此处确认的是传输关联与字节真实性，不是科学任务答案语义正确。
- `RequestOutcomeFor` 提供单个 Bytes 入参，`ConfirmResponseFor` 提供两个 Bytes 入参，透过现有静态类型验证、`WorldCall` 与 `program_host` 转换。错长 ID 拒绝。只增加查询/验收动作，不自动重启或重放请求，不修改研究者提供的 S40–S42B 活动个体程序与历史协议专家。

## 两项定向原生反例（均未运行）
1. **执行后回复在途**：真实大于U32上限的 SEND ID，合法 P2 `REMOTE_EVAL` 从 `InFlight` 到 B20 已扣费的 `CommittedB`，此刻响应仍在C→A传输，直接伪装收到响应不能验收。只有真的 Receive 之后，对相同 request ID、response message ID、完整 reply bytes 的确认才 Ok；篡改 reply 和伪造 request ID 均 Conflict。回复消费后状态仍为 `CommittedB`（不可因为邮箱空而当作未执行）。同一协议u32工作ID再次经新内核u64 SEND提交，服务去重导致本次B0；**第一次收到的合法回复不能认证第二次请求**。
2. **未执行与已处理B0分离**：真正任务漂移产生 `TaskFenceDrop`、B0，判 `Dropped`；另一个无任务栅栏世界发送无效帧，服务实际返回诊断且 B0，判 `HandledNoB` 而不是 `Dropped` / `CommittedB`。未发送ID只得 `Unknown`。

## 限制及后续
- 仍是单个可信确定性内核的 C/B 模拟，非外部 API/互联网故障恢复；依赖最近4096条审计以限制查询CPU，过旧证据安全降级为Unknown/Conflict。非加密的宿主日志不能替代跨信任域签名与事务凭证。
- `CommittedB` 代表B资源已真实消费，未确认用户任务结果；确认应由后续 S43B 活动 VM 在已有 RECEIVE 解析路径中显式调用，比较本次 ACK 和 RESULT 所对应的工作ID、请求ID和回复字节，并实施已提交但未收到时的安全停机策略。不可在未确认的 ACK 上自动重试。
- S41/S42A/S42B仍未原生确认，本次S43A也不得记为科学突破；G0 GO，G1/G2 NO-GO，E001未校准/未预注册。

[内核实现与两项反例](../../crates/world-kernel/src/lib.rs) · [VM调用契约](../../crates/organism-vm/src/lib.rs) · [主机权限映射](../../crates/world-kernel/src/program_host.rs)

## S43A.2 源消息完整性指纹修复（2026-11-21，待原生验证）
静态安全审查发现 S43A 原实现只核验 `ServiceEvent.response_message_id`、actor/source/destination 和曾在 `Receive` 中出现过的完整消息字节，却没有验证 **服务真正生成的消息字节**。研究者/受控故障注入可以在邮箱制造「同响应 ID、不同 bytes」的 Message，让它也真实经历一次 Receive，旧谓词可能误判为服务原始回应。

内核现在在服务提交时，对整个 C→A `Message` 的确定性序列化计算 `response_message_hash` 并写入 kernel-owned `ServiceEvent`；`ConfirmResponseFor` 除原因果门外还验证完整 Message hash，且不得早于其 `deliver_step`。缺 hash 的旧快照无法作为成功回复的证明（fail-closed），但可继续读取历史费用。没有赋予个体修改服务账本的权限；消息正文/时间元数据也受指纹保护。新增同测试轨迹中的强负例：伪造相同 `response_message_id` 而不同 bytes，**确实执行 RECEIVE 交付后** 再确认，必须 Conflict；对同一真实已收原消息仍可 Ok。这是一项未运行的测试预期，非验收成功。
