# OCEE S40 — 受保护远程请求被丢弃后的有限自主恢复与幂等 B 结算

**2026-11-20，开发研究/待原生验收；G0 GO，G1/G2 NO-GO；非 E001 预注册确认。**

## S39 已证实的缺口

S39 已通过真实消息队列的双重任务栅栏防止旧语义远程请求在 Send 前或服务实际消费前执行。但当 RemoteEval 已入队、公开任务随后从v1更新v2，内核按规则丢弃旧消息、不扣 B gas，此时原来的 P2 协议专家仍在有界 Receive 循环等待结果，无法恢复任务。S40 希望证明：**恢复动作由正在运行的个体程序显式发起，而非内核自动重发；恢复后应真正重新购买许可、查询 v2、重新学习并发送；原 B 请求不能重复结算**。

## 原生机制，非无限重试

- 内核在原 S39 TaskFenceDrop 中新增 actor ID，保留 message_id、发生 tick、期望版本指纹、实际任务指纹；保留单世界持久审计与旧消息不调用 B 的安全性质。新增 `WorldCall::FenceStatus()` → `Action::FenceStatus`：仅当当前 actor 在本世界确有原生S39丢弃的消息，且没有消费过恢复额度，返回可审计的 Ok + `Dropped` 载荷，收费1 A；否则Empty。它不泄露隐藏 oracle，也不无条件将超时视作失败。
- 记住实际首次写入 `/self/active` 的内容寻址 root 哈希作为 `initial_root_hash`，不能由用户传入任意新代码重启；内核每次接受重启之前重新验证 `/self/versions/{root_hash}` 的不可变规范程序字节/哈希及所属主体。新增 `WorldCall::RestartRoot()` → `Action::RestartRoot`，**只允许一次**：当前公开任务仍与历史丢弃时的新任务版本完全一致、丢弃确系当前actor、世界尚无`guarded_restarts[actor]`、原root仍存在并通过不变代码校验。成功支付8 A，重新激活原root并令专用协议 VM continuation 无效。无真实丢弃证据、任务指纹回退/变化、重复重启等必须拒绝。
- 重启不删除旧研究证据和工具注册：`/self/evidence/s36-active-probe`、`/self/tools/transform-v1`、`/self/versions/{hash}`及内核签发的原 ProbeRecord 都原样保留，要求活动VM通过S37/S38真正驳倒过期缓存并在获得新的合法 v2 探测许可后进行CAS替换。仅对明确固定的上一工作项衍生对象 `/self/results/transform-held-out`、`/self/results/s30-extra-held-out`、`/self/work/remote-values-le`、`/self/work/remote-count-le` 进行原子失效，回收对应存储空间并确保后续 root 必须重新实际生成第四B数/数量。这是**研究者预置的恢复清理合同**，非个体自行发现哪些持久数据应该保留。
- P2 专家不是被宿主杀死后自动重启：S40仅在原S23 `s23_receive_retry` **128次有界等待用尽**时，把原终点由失败修改为 `FenceStatus`，只有真的看到内核已丢弃受栅栏保护的消息后才主动请求 `RestartRoot`。由于内核切换 `/self/active` 同时清除旧 continuation，旧专家不会再继续发送旧帧。普通超时/未丢弃/不存在root不得恢复。不改动 P1 专家和既有 S38 的协议支持。
- 重新激活同一个 root 后，执行原 S38 查询额度/证据更新逻辑。只有重新认证的新 v2标签、通过有界54候选图语法选出的新程序才能生成本世界未标注输入的第四B操作数；S39 VM必须重新Arm新版本栅栏，P2专家才会再发送。原消息已经丢弃不可能再调用B服务，新消息经服务消费时新版本校验，才能使B气费24。这里的“幂等”指**这个固定开发场景下恰有一次成功的B收费**，而非一般互联网服务的 exactly-once 语义。

## 预先规定的原生对照

- **真正在途丢弃→自发恢复**：在P2 seed55任务v1中运行到确实已 Send 的 RemoteEval count4帧，记录message_id；此时研究者把真实环境规则改为v2，旧消息必须变成TaskFenceDrop且B0。P2专家有界等待后自己FenceStatus Ok与RestartRoot Ok各一次；初始root版本重新激活，S38以旧回执合法Renew收费7/第二次Probe收费52/缓存CAS v0→v1，并纠正错误程序的注册表CAS v0→v1；再次生成新第四操作数10，S39重新Arm新任务哈希，发出第二条完整4 U32远程帧。**两次真实Send，但只有新消息实际收费B24一次**，原消息永远B0；原S39不带恢复的对照维持B0。
- **实际来源P1知识的跨家族恢复**：P1 seed1先真正学得v1计算图，受费导入P2 seed55；P2第一次RemoteEval入队后研究者改环境v2，S40必须同样丢弃旧请求、从本地保留的旧工具注册中重学、付费CAS更新程序与缓存，发出的新消息B只执行一次。P1源世界的原工作应能独立正常完成B24。
- **正常无丢弃**：P2 seed55从头到尾实际完成B24，不出现FenceStatus、RestartRoot和旧消息丢弃。直接要求在无真实TaskFenceDrop时重启返回Denied，不能因人工构造动作偷得新权限。确实丢弃过但已完成一次恢复后，第二次RestartRoot必须Quota、不出现第三条成功 RemoteEval，也不二次扣B。
- **任务指纹回退时不错误重启**：旧消息已经因v2变更实际被丢弃，但研究者随后把公共任务对象字节恢复为v1；因此丢弃日志中的 observed_hash 不再等于当前任务hash，应拒绝RestartRoot、B0，不能不验证现况就重试过去的v2任务。

## 明确的科学限制

任务只存在预定义的v1/v2规则和有边界的模拟协议；P2需预置超时轮次、失败原因请求、初始root重启许可及临时数据失效表；内核自己负责保存根代码哈希与重启次数；没有分布式幂等键、跨服务交易日志、任意网络服务实际交互、任意多任务变更、重启后任务再次变化、多轮开放式恢复或生物意义的开放演化。不能把预置的失败恢复控制流程视作个体发明了恢复算法。

[S40 P2活动VM恢复](../../crates/organism-seeds/src/s40_dropped_recovery.rs) · [S40原生测试](../../crates/organism-seeds/tests/s40_dropped_recovery.rs) · [内核原生回执及单次重启](../../crates/world-kernel/src/lib.rs) · [定向 CI](../../.github/workflows/s40-dropped-recovery.yml)。

**原生数据尚待日志；不能预先把研究假设写成成功。**