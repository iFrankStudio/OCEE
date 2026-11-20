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

## 2026-11-20：原生验收及失败链（不隐藏失败）

- **[S40 定向原生 Actions #44710108675](https://github.com/iFrankStudio/OCEE/actions/runs/44710108675)** 已 **completed/success，4 passed / 0 failed / 99.83秒**；实际被验证的源码 SHA [6480ac3](https://github.com/iFrankStudio/OCEE/commit/6480ac3f986718d6f41089df74e36e5b882a4507)。原生测试覆盖P2真实在途丢弃→重学重发且B一次、P1→P2旧工具跨家族恢复、无丢弃不得重启、丢弃后任务已恢复原指纹时不得再启动。全 workspace [#44710108719](https://github.com/iFrankStudio/OCEE/actions/runs/44710108719) 另行确认，不能把定向4/4当成全量通过。
- **同世界真实恢复**：P2 seed55在v1先真实Send出4 U32 `REMOTE_EVAL` 请求，初始请求 **message_id=26**。研究者在Send后/C处理前改变公开任务和真实隐藏oracle至v2，内核发生真实 `TaskFenceDrop`，旧请求**未消耗任何B gas**。P2专家在原有128次有界接收等待耗尽后，实际发出 `FenceStatus`（确认自己确有被丢弃消息）并调用一次受费 **`RestartRoot` 8 A**，从原始冻结root重新运行，旧证据与工具保持可验证状态。活动VM真正在v2中从旧回执获得Renew许可、完成第二次Probe52 A，缓存从version0 CAS为1，原工具注册CAS更新、重构第四操作数 **10** 并建立新任务栅栏。随后新协议专家发出不同ID的第二条真实4 U32请求；独立C帧/B服务事件审计确认**仅第二条请求消耗B24**，旧request ID26零B，世界总B24且300额外tick没有重复结算。总 **A33,213 / 883 ticks / B24**，重启后 `/self/active` 单调递增到 **version3**。相同事件下未经恢复的 **S39有栅栏控制B0**，保留真实丢弃记录。
- **真正跨家族来源知识**：P1 seed1用正常活动VM实际生成v1工具（来源 **A15,415**，本身正常B24），通过两次真实受费WRITE向P2 seed55导入该工具，**import29 A**。P2先用继承工具发送原生远程请求，此时研究者改变任务使消息实际丢弃。P2自己的专家发起一次恢复，根程序对保留的旧注册/旧证据真正执行再验证，受费v2环境更新并 **CAS registry version0→1**，生成新的不同内容哈希的计算图、第四U32=10，再以新任务栅栏真正完成B请求。目标 **A32,110 / 575 ticks / B24**，全世界仍只有第二条请求被B计费。来源与目标冻结初始完整root SHA256一致：`bf7fba37c48695959ecf0b014fbf81227069222e5f8df463b548dcfcb828b839`。
- **拒绝滥用与正常对照**：无S39实际TaskFenceDrop时，`FenceStatus`返回Empty，直接请求`RestartRoot`为Denied；正常P2仍独立完成B24、不发生恢复。若S39确实丢弃消息但公开任务字节随后恢复为此前旧版本，那么当前hash与当初丢弃时的observed_hash不同，内核拒绝RestartRoot（Denied），B0；曾合法恢复一次之后再次请求必须Quota。snapshot重载哈希一致，旧消息绝不能因重新恢复被第二次消费。
- **显式版本化而非复位规避CAS**：S26原路由硬编码`/self/active`预期版本0，S40在真正重启后版本为2。内核新增只读、受费的 `/sys/active-version`，活动S40 root通过正常 `WorldCall::Read` 获得当前真实版本，再用普通`WorldCall::Write`原子CAS把活动专家从版本2切换到版本3。没有重置版本号或跳过内核的写前验证；这不是外界直接把答案植入根程序。
- **有限清理表的代价**：为了让真实root重复执行，受控重启只失效一组研究者明确认定的本任务派生状态：前次transform held-out回执/额外预测、两份远程B操作数/数量工作对象、以及预先写入但必须按当前回合重新发现的`/self/discoveries/framing-family`。旧的不可变程序版本、工具注册、可信内核ProbeRecord与本地证据缓存不能删除，用于真正检测失配和CAS修复。该硬编码清理表不是通用记忆/状态自动分类机制。

### 保留的真实失败与修复
- [#44706517834](https://github.com/iFrankStudio/OCEE/actions/runs/44706517834) 初版长回放因调整测试上限取消；[#44707304132](https://github.com/iFrankStudio/OCEE/actions/runs/44707304132) **2/4通过**，两组恢复B0。后检查发现 S26路由程序在重启时仍把`/self/active`期望版本固定为0，因此新增可受费读取的真实版本，初次/二次切换CAS分别匹配0/2。
- [#44708193096](https://github.com/iFrankStudio/OCEE/actions/runs/44708193096) 修正活动版本后仍2/4，显示恢复中还有第二阻塞；[#44709096078](https://github.com/iFrankStudio/OCEE/actions/runs/44709096078) 为增强审计诊断新增的测试格式化错误 `E0277 Code缺少Display`，在 [1c3ee58](https://github.com/iFrankStudio/OCEE/commit/1c3ee589caee74447f2a6a7a53824e966a7ed2a9) 仅改为Debug格式，未改变世界行为。
- [#44709246739](https://github.com/iFrankStudio/OCEE/actions/runs/44709246739) 的原生详尽审计明确显示：P2已真实RestartRoot（active version2）、完成第二次Probe（count2）、证据CAS（v1）、注册表CAS（v1）、新操作数WRITE、Arm新栅栏，却在随后 `Write(/self/discoveries/framing-family)` 得到 **Conflict**，最终B0。根因是第二次回合仍保留了仅允许创建一次的派生协议分类对象。修订 [45c106e](https://github.com/iFrankStudio/OCEE/commit/45c106e3153d2c9ad00d42b9e4ee4cfecbe7c090) 只将其纳入受控清理，不删旧工具或回执。
- [#44709627497](https://github.com/iFrankStudio/OCEE/actions/runs/44709627497) 已 **3/4通过**，跨家族旧工具→重学→B一次全链原生成功；剩余失败是消融测试错误地使用**无栅栏S38**为对照，却断言它在任务漂移后B0。实际上S38仍合法计算旧语义并支付B24，这正是S39原先发现的问题。修订 [6480ac3](https://github.com/iFrankStudio/OCEE/commit/6480ac3f986718d6f41089df74e36e5b882a4507) 把对照改成确实有栅栏但不会恢复的S39，未改环境、种子、VM恢复逻辑或成功标准，随后 **4/4通过**。

### 结论与必须保留的界限
- **可以支持**：在研究者固定的单世界P2有界故障模型、v1/v2已知升级协议、一次合法续期、两次最多环境Probe，以及由冻结P1/P2协议专家运行的真实本地C/B模拟条件下，活动P2专家能从**真实消息丢弃记录**触发一次有费重启，重新向v2环境获取可信标签，CAS修复旧知识并成功重发，最终B服务恰好对新任务计费24且不重复执行丢弃的旧消息。原生源码能编译、验证并在至少两种来源条件下运行。
- **不能支持**：通用异步失败识别、任意内存分类、服务端生产级幂等键、真正分布式 exactly-once 事务、多工作项并发恢复、任务多次变更/多轮自我强化、未知API自动学习、开放式演化或有赛博生命形成。当前128轮重试、原root存储、一次重启、清理清单、54候选语法、两条专家和任务v1/v2都是研究者先验。G0 GO；**G1/G2 NO-GO**；E001尚未预注册。下一项S41可探索减少固定超时与恢复清单，并通过内核任务事件/状态订阅做更即时、可收敛的因果通知；但不得在模型中自动替代个体作决策。