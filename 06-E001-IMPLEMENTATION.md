# OCEE-0 / E001 可执行协议与实验实施契约

- **文档编号**：OCEE-EXP-006
- **版本**：0.2.0
- **状态**：Implementation Ready for calibration；**NOT pre-registered**，未执行确认性实验
- **上游**：[03-E001-PROTOCOL.md](./03-E001-PROTOCOL.md)、[04-ENGINEERING-CONTRACTS.md](./04-ENGINEERING-CONTRACTS.md)、[05-INITIAL-ORGANISMS.md](./05-INITIAL-ORGANISMS.md)

## 0. 目的与界限

本文件把 E001 的节点 C 协议、节点 B 服务、任务与对照流程转为可执行约定。下面的例子是**研究者可见的参考规格，不是个体初始输入**。协议生成器代码和独立求解器绝不能被当作个体的预置程序或普通可读世界对象。

E001 的研究问题：带通用探索能力的 S 型个体是否能在未知协议中形成、复用、迁移一项**实际新增**的程序能力；O 型的自发探索起源另测。

## 1. 服务端状态与请求语义

C 是唯一公开远程入口；B 无法被个体直接寻址。C/B 均由确定性状态机模拟，禁止真实套接字、进程执行和文件 IO。

操作语义固定；协议家族改变的是**请求编码/分帧/标记**，而不是外部评价标准。

| 语义操作 | 输入 | 输出 | 说明 |
|---|---|---|---|
| `HELLO` | 空载荷或合法 nonce | 服务实例公开标识、协议版本提示 | 只确认服务可达，不返回真实 opcode 映射 |
| `CAPS` | 空载荷 | `FOLD32` 能力摘要、负载上限、结果方式 | 表示服务能力，不提供完整帧构造器 |
| `REMOTE_EVAL` | `request_id:u32`、`count:u16`、`count × u32` | 已接收/拒绝与请求编号 | C 校验后交由 B 按容量异步执行 |
| `RESULT` | `request_id:u32` | pending / error / `u32` 结果 | 只有对应已授权请求能读取 |

**跨家族请求编码约定（消除 request_id 歧义）**：逻辑请求均有 `request_id:u32le` 与 `operation_body:bytes`。P0/P1 的 `payload = request_id:u32le | operation_body`；P2 的 `request_id` 单独放在 `req_tag` TLV，`body_tag` 中只放 `operation_body`，禁止再重复编码编号。HELLO 的 operation_body 为可选 `nonce:u32le`；CAPS 为空；REMOTE_EVAL 为 `count:u16le | count × u32le`；RESULT 为空。所有正常响应的逻辑 body 以 `status:u8` 开头（`0=OK,1=PENDING,2=ERROR`），其后按操作附带 `request_id` 关联的服务信息或 `result:u32le`。P0/P1 响应同样在 payload 首部放 request_id；P2 放于 req_tag。畸形帧无法被正常协议解析时，C 通过**传输层受限诊断消息**（而非假装合法协议帧）回送错误码和接收消息 ID，不泄漏真实私钥。

B 唯一需要实现的工作负载 `FOLD32`：对按序输入 `x_0 ... x_(n-1)`，计算
`acc_0=0; acc_(i+1)=(acc_i + rotl32(x_i, i mod 32)) mod 2^32`。
约定 `1<=n<=64`，数据 u32 little-endian；结果 u32 little-endian。研究者参考计算器必须是独立实现，不能重用 C/B 的同一函数来“验证自己”。

`REMOTE_EVAL` 成功不等于计算完成，`RESULT` 的有效返回才证明使用 B。使用 B 消耗 B 的 gas；C 的消息解析、队列和流量占用独立计费，所有成本在 manifest 中按资源类型分别报告。

## 2. P-Family v1 字节级语法

生成器输入：`family`、`seed`、`generator_version`，输出同一 episode 固定的私有协议参数。 参数采样用域分离的确定性协议 RNG（当前 P3 代码使用带版本号的固定 SplitMix64 派生（非密码学安全算法），必须在生成器 manifest 中锁定版本和种子域；此伪随机算法只保证可重放，不作为访问控制或秘密保护机制）；同配置的参考向量字节必须跨运行一致。所有标记与 operation code 必须在其家族的已声明有限集合中选取且不能互相冲突；请求与响应共用分帧家族，通过方向/消息类型字段区分。整数端序写死，不允许由平台决定。

### 2.1 P0：定长头 + XOR 检验

```text
FRAME = marker:u8 | kind:u8 | opcode:u8 | len:u16le | payload:bytes[len] | check:u8
check = xor(all bytes from marker through end of payload)
kind: request=0, response=1   (固定基础格式)
marker = seed_choice([0xA1,0xA3,0xA5,0xA7])
opcode = 由协议 RNG 从 0x10..0x7F 无放回抽取的 4 个唯一值，依 HELLO/CAPS/REMOTE_EVAL/RESULT 顺序赋予；具体数值隐藏
```

最长载荷 1024 bytes；长度不符即 `BAD_LENGTH`，校验不符 `BAD_CHECK`。响应需携带关联请求编号；无法解析的探测返回受限错误事件，不返回未授权的远端状态。

### 2.2 P1：分隔符 + 转义 + 校验

```text
FRAME = 0x7E | ESCAPE(inner) | 0x7E
inner = kind:u8 | opcode:u8 | len:u16be | payload:bytes[len] | check:u8
check = xor(inner bytes preceding check) xor seeded_xor_key
ESCAPE: 0x7E -> 0x7D 0x5E; 0x7D -> 0x7D 0x5D
kind: request=0, response=1
seeded_xor_key = seed_choice([0x00,0x33,0x55,0xAA])
opcode = 从 0x10..0x7F 无放回抽取的 4 个唯一值，按 HELLO/CAPS/REMOTE_EVAL/RESULT 赋予
```

先验证分隔符和转义完整性，再检验长度和校验。非法转义 `BAD_ESCAPE`；解析失败返回的是局部阶段反馈，不直接告知密钥或 opcode 值。

### 2.3 P2：嵌套 TLV

```text
FRAME = marker:u8 | outer_len:u16be | TLV(tag=0x10, value=CHILDREN)
TLV = tag:u8 | len:u16be | value:bytes[len]
CHILDREN = permuted TLV(kind_tag,kind:u8)
         + TLV(op_tag,opcode:u8)
         + TLV(req_tag,request_id:u32le)
         + TLV(body_tag,payload:bytes)
         + TLV(check_tag,check:u8)
check = xor(bytes of children before check field in canonical tag order)
marker = seed_choice([0xD1,0xD3,0xD5,0xD7])
(kind_tag,op_tag,req_tag,body_tag,check_tag) =
    seeded distinct permutation of [0x21,0x22,0x23,0x24,0x25]
opcode = 从 0x10..0x7F 无放回抽取的 4 个唯一值，按 HELLO/CAPS/REMOTE_EVAL/RESULT 赋予
```

P2 的字段实际排序由 seed 决定，但校验计算统一按 `kind,opcode,request_id,body` 四字段的 `tag | len | value` 顺序（不含校验字段）；未知字段拒绝，重复字段拒绝，必须恰有五字段。P2 与 P0/P1 不只是常量替换：需构造嵌套结构、处理字段顺序及长度。

**强制校验向量**：协议生成器输出每个家族至少一个合法请求、合法响应、错误长度、错误校验、非法 opcode、畸形嵌套向量，并由独立解析器复验。发布前必须通过这些向量的确定性重放。

## 3. 反馈、心跳与可学习性

C 每 256 个世界 step 在只读公开对象 `/public/C/hello` 发布一对**合法的 HELLO 请求/响应示例帧**及发送/响应相关性；示例允许被复制，但不直接提供 REMOTE_EVAL 的完整封装。样例随当前 episode 的协议生成；同一次正式实验内样例语法一致。

帧完整且语法有效时，正常响应使用所在家族的编码；帧不完整/格式错误时，C 的传输层诊断记录附在该次入站消息的反馈上，以 `message_id, stage, code` 标识，可被发起者在授权邮箱读取，不需要预先会解码当前家族的完整响应。此诊断面仅披露下面的有限标签，不能作为另一套替代远程计算协议。

错误反馈统一为有限标签：`BAD_FRAME`、`BAD_LENGTH`、`BAD_ESCAPE`、`BAD_CHECK`、`BAD_FIELD`、`UNKNOWN_OPERATION`、`BAD_BODY`、`UNAUTHORIZED`、`SERVICE_BUSY`。可附加上限为 255 的故障字段序号/阶段，**不得**返回期望 marker/opcode、checksum key、隐藏 seed 或完整正确请求。

未知协议应既有不确定性也有可推断性：

- P0/P1/P2 均提供可观察的合法边界示例、受限解析反馈、若干不同输入的远程任务机会；
- 不允许只靠猜一个 128 位随机秘密才能成功；seed 选择的是有限可比较的语法变体，而非无反馈的密码学谜题；
- 若 S 和独立参考求解器在默认预算内均无法构造有效请求，则必须先校准环境，而不能把这种失败认定为生命不具有学习能力；
- 不允许开发集和正式集使用同一协议 seed，且不得将正式 P2 专用解析代码放入 S 程序。

## 4. 任务发布与远端收益

驱动器将工作项发到个体可读的任务邮箱。每项包含 `job_id`、`values:u32[]`、`expected_output_schema=u32`、到期 step 和可公开验证的计算语义；不包含“请调用 C”或目标协议正确报文。

起始候选：每个 episode 在 step 0 至结束前均匀发布一系列 `n=32/64` 的 `FOLD32` 工作项；本地 A 可以在足够资源时自行计算，远端 B 有更高吞吐可被利用。**具体工作项密度、A/B gas 成本函数、C 解析费/链路费须由校准确定，不能直接当成已测收益**。资源下降事件在 step=8192 的 tick 开始前生效，默认 A 从 `12000` 改为 `4000 gas/epoch`。

工作项外部完成验证由独立评价器执行。个体只接收环境原生响应和工作项内容，不获得 `success` 专用奖励或评价器排名。

## 5. 异步请求与权限

1. 个体 `SEND(C,frame)` 入队，按正常通信字节预算计费。
2. 消息在 `deliver_step >= sent_step+2` 递送，由 C 解析；C 记录错误/接受信息作为模拟服务事件。
3. 若请求合规，C 代为调用 B；B 完成后产生响应/可供 RESULT 读取的条目；不允许直接读取 B 的内部对象。
4. 请求标识必须属于授权个体；重复 `request_id` 的幂等/冲突行为固定：同 payload 重放返回相同状态，异 payload 返回 `BAD_BODY`。
5. 发生服务版本变化时，以服务**处理请求时**当前配置为准；旧请求如果已接受则继续原版本处理并可返回结果。
6. 队列、任务、C/B 缓存和服务随机状态必须属于 `WorldState`，受快照回放约束。

## 6. E001 四组、episode 和状态继承

依 [05](./05-INITIAL-ORGANISMS.md) 的 A/B/C/D 机制开关，各组共享同一 S 型初始程序、世界种子、任务序列、权限、配额上限及 VM 指令费。

| 阶段 | 上限（世界 tick） | 情况 |
|---|---:|---|
| 发现 | 40960 | 单一协议家族；可通过 C 探索 B |
| 迁移 1 | 16384 | 同家族未见实例 |
| 迁移 2 | 16384 | 跨家族未见实例 |
| 迁移 3 | 16384 | 至少一个与迁移 2 结构不同或不同参数的新实例 |

共 90112 ticks/seed/组。每个 episode 新建世界，包括 A/B/C 服务实例；D **只**继承其自建程序、工具、许可范围内的个体记忆；C 不继承新经验，B 不继承新经验，A 初始不变。新世界的权限和资源账户重新建立，不沿用前一 episode 的能力令牌。跨家族迁移的正式测试至少包含一个训练中未注入完整解析器的家族。

**同一 seed 的配对**由 `master_seed -> domain_separated(world,protocol,workload,agent)` 派生；不同组复用相同 world/protocol/workload 子种子，仅 agent 子种子依预注册规则处理，避免组间遇到难度不同的协议。

## 7. 独立可学习性校准与对照

实现后必须先运行开发/校准集（>=8 个独立 seed），至少验证：

1. 独立参考构造器根据完整私有语法可以生成正确请求，并由另一份参考解析器确认；这是服务实现正确性校验，**不能**证明可学。
2. **信息受限的参考求解器**只能获得与个体相同的可见心跳、错误和动作预算，不得接收协议私钥；其目标是证明有限交互可以在预算内发现有效协议使用法。
3. 固定初始策略、无反馈随机变异和反馈学习的开发集结果用于检查任务是否过易或过难；任何基于开发集进行的调整均必须升级配置版本。
4. 校准本地 A 和远端 B 工作量：B 有真实的可用吞吐优势，且学习成本在足够任务量下有可能摊销；报告 A 消耗与 B 消耗，不虚构单一“总能量”。
5. 检查错误反馈不会直接泄漏正确 opcode、帧模板或正式 seed。检查 P2 在没有 P2 专用解析器的 S 版本上不是结构性不可能。
6. 所有开发 seed、日志及参考求解器版本在校准报告中保留，不计入确认性结论。

如果独立受限求解器无法在预算内成功，必须记录 `calibration_no_go` 并修改环境，**不允许**对原目标宣称失败或成功。

## 8. 预注册与正式集封存

正式运行之前：

- 固定 `kernel_version, vm_version, cost_table_version, protocol_generator_version, evaluator_version`；
- 冻结 S 初始程序真实哈希、四组机制开关、协议家族/参数空间及预期功能类别；
- 固定 >=30 个配对 seed，每 seed 4 组，开发/正式 seed 完全隔离；
- 封存正式 seed 清单与生成参数的摘要；秘密值只进入受限运行配置，不应泄漏到个体可见日志或公开存储；
- 冻结 03 文档的成功门槛和统计脚本，包括失败样本、删失成本的报告规则；
- 审阅“未见家族”的定义，确认未因开发或初始代码提前接触其专用实现。

只有完成上述步骤，03/E001 才可以把状态改成 `Pre-registered`。目前 **不能** 将 0.2.0 的工程规范误写成已经完成校准或正式试验。

## 9. 核心验收与工件

只需维护关键端到端测试：

- 三家族合法/非法消息向量及 B 独立计算结果；
- SEND 延迟、协议版本变更和请求幂等；
- 不可直连 B、权限令牌隔离、无宿主网络/文件访问；
- 初始能力清单及四组跨 episode 清理；
- 确定性重放、隐藏 seed 不泄漏；
- 有意义工具的两次调用、因果来源及移除工具反事实。

工件结构沿用 03 的 `experiments/E001/{configs,runs,reports}`；新增 `protocol_vectors/`、`calibration_solver/` 与 `initial_manifests/`。不要提交巨量原始快照到 Git；保留不可变存储工件、摘要与可追溯的代码版本。

**下一研究闸门**：原型代码能执行并通过核心不变量验收 → 独立可学习性校准通过 → 正式配置冻结 → 才能执行 E001 确认性实验。
