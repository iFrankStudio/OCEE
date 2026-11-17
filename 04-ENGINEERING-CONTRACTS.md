# OCEE-0 工程契约：内核、VM、状态与确定性

- **文档编号**：OCEE-ENG-004
- **版本**：0.2.0
- **状态**：Implementation Ready（仅工程接口与实现起点；未完成 E001 校准）
- **上游**：[世界规范](./01-WORLD-SPEC.md)、[个体规范](./02-ORGANISM-SPEC.md)
- **决策记录**：[08-DESIGN-DECISIONS.md](./08-DESIGN-DECISIONS.md)

## 0. 规范优先级与术语

本文件是 **OCEE-0 v0.2** 的执行语义唯一来源。与 01/02 旧段落冲突时，必须先修改旧段落，不允许各模块自行解释。MUST 为验收必须条件，SHOULD 为建议。独立评价器不是世界内个体可调用服务。

- **tick/step**：世界驱动器的一次调度机会，编号从 0 开始；一次最多处理一个个体外部操作意图。
- **VM microstep**：VM 内的一条指令；消耗 gas，**不**递增世界 step。
- **ActionIntent**：活动程序通过 `WORLD_CALL` 让出执行权时提交的一个六原语操作请求；它尚未被世界执行。
- **EXEC**：六原语之一，调用另一段受限程序进行隔离、无真实外部副作用的试运行。它**不是**宿主驱动活动程序的入口。
- **episode**：有独立世界实例与预算的一次实验；继承清单由 06 文档规定。

## 1. 唯一权威状态与数据所有权

```rust
WorldState {
    schema_version: u32, kernel_version: String,
    step: u64, rng_state: DeterministicRng,
    epoch_index: u64, nodes: BTreeMap<NodeId, NodeState>,
    objects: BTreeMap<ObjectId, VersionedObject>,
    organisms: BTreeMap<OrganismId, OrganismState>,
    mailboxes: BTreeMap<MailboxId, VecDeque<Message>>,
    pending_deliveries: BTreeMap<(u64, u64), Message>,
    events: Vec<ScheduledEvent>, next_sequence: u64,
    security: HostSecurityState
}
OrganismState {
    id: OrganismId, active_program_hash: Hash,
    program_store: BTreeMap<Hash, CanonicalProgram>,
    memory: BTreeMap<Key, Vec<u8>>,
    tools: BTreeMap<ToolId, Hash>,
    vm_continuation: VmContinuation,
    latest_feedback: Option<ActionResult>,
    rng_state: DeterministicRng,
    status: Idle | Ready | Waiting | Faulted,
    resource_account_id: AccountId
}
```

`WorldState` 是唯一持久化权威；`OrganismState` 是其子结构，**不能**另建会与世界快照冲突的个体数据库真相。日志哈希和候选来源存于宿主不可变的事件存储，个体只能读获授权的投影。消息、定时任务、隔离调用消耗记录和 VM continuation 都必须包含在快照或由日志完全可恢复。

可使用 SQLite 事务保存 `snapshot + log_cursor`，但执行内存中的状态需先复制/计划，再一次性原子提交；崩溃恢复时只能选择最后一个已提交点。存储回收不允许删除被活动版本/工具引用的程序对象。

## 2. 单一 tick 状态机（固定顺序）

给定当前 `step=k`：

1. **前置事件**：按 `(due_step, priority, sequence_no)` 稳定排序并应用所有 `due_step == k` 的事件；将 `deliver_step <= k` 的待投递消息移入邮箱。
2. **epoch 授予**：若 `floor(k / epoch_length)` 与已记录的 `epoch_index` 不同，按**本 tick 生效的配置**重置各账户本 epoch 可用 gas / 通信吞吐。初始 `k=0` 也进行一次授予。未使用额度默认不结转。
3. **调度**：若个体可运行，用 VM 恢复 `active_program + continuation + latest_feedback`，执行直到：产生一个 `ActionIntent`、显式停止、gas 用尽或指令上限触发。VM 微指令 gas 由 A 计费；这一阶段不改变外部世界状态。
4. **动作执行**：对一个 `ActionIntent` 验证权限、参数、版本和预算；计算动作世界侧成本。成功则提交其状态差量并计费；失败则无部分外部写入，只计事先公布的实际解析/探测费。若 VM 未发出意图，该 tick 不发生个体外部操作。
5. **反馈入队**：把结果加入该个体的 `latest_feedback` 或授权邮箱，只有后续 tick 的活动程序可以消费；`SEND` 的成功只代表请求入队，C/B 处理结果由独立确定性服务事件产生。
6. **落盘**：单事务记录有序事件、实际成本、状态差量和状态哈希，令 `step=k+1`；失败也递增一次 step。

无个体活动时世界不自动前进。**仅实验驱动器**可显式发起 `NOOP` tick 以推进环境/消息时序；NOOP 不属于个体可用原语，也不产生“自主活动”。驱动器 tick 同样计入 episode 步数预算。

例：`epoch_length=1024`，发生在 `due_step=8192` 的 `RESOURCE_CHANGE(A,4000)` **先**应用，且本 tick 从 4000 gas 开始新 epoch。不得同时实现“动作后应用事件”的另一套规则。

## 3. 活动 VM 与显式 EXEC 的区别

活动程序由宿主在第 3 阶段调度，并非作为一个 `EXEC` 世界动作被反复递归调用。活动 VM 执行 `WORLD_CALL(op,args)` 时执行 `YIELD`，一个 tick 最多产生一个意图。下一 tick 注入 `ActionResult {code,payload,cost}` 后，继续当前 continuation。

显式 `EXEC(program_hash,args,gas_limit,isolated=true)` 是一个独立世界动作：

- 仅运行**纯计算及获授权本地快照只读**的候选程序；内部 `WORLD_CALL` 不得写入共享状态、发消息或调用 B/C。遇到副作用意图返回 `ERR_ISOLATED_SIDE_EFFECT`。
- 候选的输出、错误和消耗 gas 可被观察。每条候选 VM 指令依旧计费，世界 step 只增加一次。禁止嵌套 `EXEC`。
- 真实远程协议实验通过活动程序的 `SEND`、`RECEIVE` 进行，按真实通信与资源成本计费；**隔离 EXEC 不能变成免费探测协议的后门**。

活动程序可以生成新版本并以带 `expected_version` 的 `WRITE /self/active` 原子切换；切换在该动作提交后对下一次调度生效。无效版本不得切换。故障时宿主恢复只能标记为 `host_rescue`，不能冒充自主修复。

## 4. 程序格式与执行边界

程序采用**规范化的类型化 SSA 基本块/控制流图**。基础类型：`Bool, U32, Bytes, Unit, Result<T>`；所有操作有显式类型。版本 1 强制：

- 模块含 `format_version, entry_function, functions, blocks, instructions`；函数/块 ID 唯一、引用必须已声明；基本块由 `BRANCH, JUMP, RETURN, YIELD` 之一终止。
- 调用栈深度最大 32，`REPEAT_BOUNDED` 上限 256，单次活动 VM slice 最大 2000 gas；程序大小默认 64 KiB，结果最大 1024 bytes；**这些数值均为校准前上限**。
- `ADD/SUB` 使用 U32 模 2^32 计算；`SLICE` 和 `BYTE_AT` 越界返回类型化错误，不 panic；`MOD 0` 返回 `ERR_ARITH`；Bytes 连接与内存分配同样受预算。
- 指令表最小集合沿用 02 文档；新增控制流 `YIELD` 是 `WORLD_CALL` 的规范化终结形式，不是新的世界原语。
- 规范序列化使用严格 UTF-8 JSON，键按字典序、整型十进制、字节以小写十六进制、无浮点数和未声明字段；程序哈希 `SHA-256("OCEE-PROGRAM-V1\0" || canonical_bytes)`。实现可提供二进制缓存，但**其哈希以规范字节为准**。

静态验证器拒绝未定义引用、类型不匹配、无界控制流、非法宿主调用。动态 gas 硬限制是第二道边界；验证器不得根据候选性能替个体决定采用哪一版本。

## 5. 六原语请求/错误模型

```text
ActionIntent { actor_id, op, arguments, expected_version?, request_id }
ActionResult { request_id, code, payload, vm_gas, action_gas, bytes_charged }
```

`OBSERVE(target)`、`READ(id,range)`、`WRITE(id,expected_version,bytes)`、`EXEC(hash,args,limit,true)`、`SEND(dest,payload)`、`RECEIVE(mailbox,max_count)` 为全部公开操作。

错误码统一：`ERR_DENIED, ERR_UNKNOWN, ERR_NOT_FOUND, ERR_CONFLICT, ERR_QUOTA, ERR_GAS, ERR_BANDWIDTH, ERR_EMPTY, ERR_INVALID_CODE, ERR_ARITH, ERR_ISOLATED_SIDE_EFFECT, ERR_UNAVAILABLE, ERR_BAD_REQUEST`。

- 拒绝越权操作时返回不泄漏内部对象存在性的通用拒绝；按已公开成本表计费。
- 所有 `WRITE` 采用 CAS；`/self/versions/<hash>` 只允许首次写入且验证内容哈希；`/self/active` 仅写已有已验证的版本引用。
- ID 从状态中的单调序号产生，权限来自能力凭证而非 ID 猜测；个体凭证只允许授权范围。
- 单条消息最大 1024 bytes，`SEND` 需验证发送方对 C 的可达性和字节预算。C 处理完成后产生新的消息事件；送达顺序以 `(deliver_step,sequence_no)` 决定。

## 6. 资源计量与确定性约束

计费分三类独立记录：`vm_gas`、`world_action_gas`、`comm_byte_hops`；不得将不同资源强行相加。世界检查可用总额后提交；失败操作不得越过资源上限；超额故障不能清空持久状态。

- 所有 RNG 必须是由实验 seed 派生的确定性流；`rng_state` 快照包含调用计数。建议按 `world / organism / protocol / workload` 分域派生，避免增减某一模块调用改变其他模块试验参数。
- 规范状态哈希涵盖完整授权无关的物理状态、RNG、队列、资源与调度状态；快照摘要与日志链摘要分别保存。
- 使用整数和显式稳定排序；任何墙钟、线程竞态、系统环境变量、宿主文件枚举或非记录网络输入均不得影响状态转移。
- 资源计费表有 `cost_table_version` 和配置哈希；正式实验开始前必须校准并冻结。

## 7. 实现接口与验收样例

建议 Rust crate：`world-kernel`（状态与 tick）、`world-resources`（费用与权限）、`organism-vm`（验证和解释）、`protocol-sim`（C/B 服务）、`event-store`（事务与重放）、`experiment-runner`（外部驱动）。以上为模块职责，不要求一开始拆成多个独立 crate。

关键验收向量：

1. `step=1023` 额度不足的动作被拒；到 `step=1024` 开始新 epoch，额度按配置重置。
2. `due_step=8192` 的资源变化在该步 VM 执行前生效。
3. `SEND` 于 `k` 入队，设置 `deliver_step=k+2` 的响应不得在 `k+1` 消费。
4. `EXEC` 内尝试 `SEND` 返回 `ERR_ISOLATED_SIDE_EFFECT`，不产生 C/B 消息、仍收取已运行的 gas。
5. `WRITE /self/active` 验证失败或 CAS 冲突后，旧版本哈希和 continuation 保持可恢复。
6. 相同快照、种子和动作流的最终状态/事件摘要完全相等，改变任一已处理动作导致可解释的分叉。

仅为上述关键不变量、协议正确性和对照组状态清理编写自动测试；不为每个实现细节增加重复测试。

## 8. 明确不在本阶段解决

意识与个体身份本体论、无限开放演化、多生命社会、真实网络与物理执行器。世界内核提供的安全隔离不会被报告为个体自主能力。
