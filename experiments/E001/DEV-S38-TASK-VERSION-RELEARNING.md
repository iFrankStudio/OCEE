# OCEE S38 — 任务版本变化后的有界主动再查询与知识恢复

**2026-11-20 · 开发性机制实验，非正式 E001 确认。当前 G0 GO，G1/G2 NO-GO。**

## 研究动机及科学命题
S37 已通过原生因果对照确认：本地证据可被提前伪造、查询后CAS篡改，或者在环境任务真实变更后过期；旧S36会错误使用该证据并合法完成错误语义的 B 运算。S37 以可信内核回执签发记录+当前任务哈希通过受费 `VerifyProbe` 防御这些情况，但只能停止、不能主动恢复。S38 检验在明确的预算和任务升级协议下，**运行中的 VM 能否证明原查询确实存在、取得一次受费的重新查询许可、以 CAS 更新陈旧缓存，重新验证新任务标签、学习新程序并继续实际 C/B 计算**。

## 明确的研究者先验与边界
- 新任务升级协议明确固定为开发性任务 `/public/E001/compose-depth` 的 **v1→v2**。v1 task_id `s36-interactive-probe-v1`，隐藏 oracle `hidden_compose(seed,input)+stages×Len(input)`；v2 task_id `s37-changed-rule-v2`，公开对象 `version=1`，隐藏 oracle 更改为 `2×Len(input)`。v2 不是个体自己创建或控制的任务；研究者通过测试宿主授权的世界变更实现，普通 `/self` WRITE 无法修改 `/public` 对象。
- v1 与 v2 的两个公开示例 `[1]`、`[2,0]` 在数字标签上刻意相同，**单靠它们不能识别算法换了**。源 P1 对 v1 真正学得的 XorFold+Len 类可执行程序跨家族导入 P2 后，仍可能拟合新任务的两个标签，但对 `[7,8,9]` 和未见测试输入有错误语义。新任务正确规则 `2×Len` 落在研究者先验54个图组合的旧固定词汇内，因此**不证明跨不兼容语法的自由发明**。
- 世界仍在本地确定性协议模拟器内，P1/P2 专家和真实 B FOLD32 任务格式仍研究者预置；探针候选 `[1]`、`[7,8,9]`、Len/XorFold 比较策略、54候选搜索/修复和CAS代码在初始根里都是先验，没有开放互联网、动态第三方 API 或自动调整候选语法。

## 新的受费授权接口及权利范围
- 内核新增 `WorldCall::RenewProbe(Bytes)` -> `Action::RenewProbe{actor,evidence}`，允许传递**个体已持有的原始旧查询回执**。授权条件全部成立才成功：公开开发任务是明确 v2 task_id、公开对象 version1、C/B本地服务存在、之前真正成功探测过1次，内核仍持有 v1 原始签发回执、旧回执字节完全匹配内核的旧记录、旧任务指纹与当前任务指纹不同、且以前**从未**续期成功。
- 对合格版本的授权检查收取 **7 A gas**（成功及回执不符），拒绝未经指定版本的任务、空历史额度、重复授权，不泄露环境隐藏 oracle。成功仅把 `probe_renewed_for=当前task_hash` 写入 world 内核状态，不直接返回新标签、不会发起第二次查询，也**不自动重置全世界预算**。
- 原始 `Probe` 配额被严格延长为最多 **2次成功查询**：首次可以在 v1进行；第二次仅在 Renewal 成功，且 `probe_renewed_for` 与当前任务字节哈希仍完全一致时允许。新增 v2 查询再次收取原费用 `40 + 4*input.len()` A gas（对 `[7,8,9]` 为 **52 A**），更新 `probe_count=2` 和内核签发记录；第三次查询仍必须返回 Quota，任务再次改变也不会获得不受限额度。
- `probe_renewed_for` 及新版回执与原世界审计链同时持久化到 snapshot/state hash，默认空值省略序列化保持旧未续期世界兼容。S37 未经显式续期对 v1→v2 的原 `Probe` 依旧返回 Quota；不能把v2任务变更误认为自动免费授权。

## 活动 VM 的一条不可循环的恢复路径
- 在 S37 旧缓存 READ -> 真实 `VerifyProbe` 后，只有 `Conflict` 进入 S38 的任务续期尝试；`NotFound`（没有内核签发历史）与其它错误全部按旧机制 fail closed。S38 从原缓存的实际 READ 回执中提取当时取得的候选标签字节，向内核调用受费 `RenewProbe`；**伪造、被篡改的缓存不能证明旧回执**，因此不能拿新配额。
- `RenewProbe` 成功后，活动 VM 沿研究者给定的相同 Len/XorFold 区别度策略再次选择 `[7,8,9]`，真实 `Probe`，获得 v2 新标签，然后对已有自有对象 `/self/evidence/s36-active-probe` 作受费 **`Write(expected_version=Some(0))` CAS**，版本从0变1；不能使用不检查版本的隐式覆盖。
- 第二次查询后，活动 VM 使用**单独的最终 `VerifyProbe` 静态区块**验证新写入的原始环境回执，成功才继续用该标签执行当前候选；失败直接停止。第一次实现曾错误地返回到原S37可续期入口，VM静态验证正确发现 `unbounded control-flow cycle`，后改为终结分支，**以程序图结构限制最多一次续期**，而不靠运行时偶然停止。
- 正常旧程序若与新探针不符，S30既有有界54候选搜索必须重新运行，候选所执行的真实 `Exec` 对照来自新任务查询的标签，代码重新生成并按真实付费CAS更新注册表v0→v1；新程序输出来自目标世界未见输入，是实际 C/B 的第四个 U32，必须同时经研究端独立 oracle 和真实 B fold32 核验。

## 原生验收与反事实
- **原生目标P2世界中途v1→v2**：活动VM先自主真实Probe得到v1回执并付费WRITE缓存，研究者随后真实改变公开任务及内核 oracle。S37不续期控制遇 Conflict 停止 B0；S38必须真实发生1次 `RenewProbe`（费用7 A）、第2次成功 `Probe`（费用52 A），真实CAS自有缓存version0→1，再受费验证新回执，最终程序第四操作数等于v2真实规则 `2×Len(held_out_input)=10`，真实B24且世界快照状态相等。
- **跨家族错误知识恢复**：source P1 seed1 在v1真实学习并保存旧组合程序，按正常受费世界WRITE导入target P2 seed55；目标v1查询后研究者改变其规则为v2，旧P1程序因新的主动证据而失配，被目标重新学习的v2程序替代，注册表真实CAS v0→v1，仍完成实际 C/B。此对照用于确认**恢复不依赖宿主替换程序或把新规则直接塞给VM**。
- **污染证据与预算拒绝**：凭空伪造的本地cache没有真实签发记录，S38必须跟S37一样拒绝（B0），不能借任务更替购买查询额度。即使之前合法Probe，旧cache通过真实收费CAS被篡改后，续期时因为回执字节不匹配内核签发记录，必须 `Conflict`，不能续期或做第二次环境查询。
- **直接内核防滥用**：初始v1禁止RenewProbe；v2篡改的旧回执不能授权；从v1成功Probe并升级到指定v2时正确回执授权一次、收费7；新的v2 Probe必须收费52、返回新的环境规则、更新两次查询的状态；第二次授权/第三次Probe必须Quota且无费用；快照恢复保留全部状态。

## 风险与避免过度解释
该授权机制是**为研究任务预先设计的 v1→v2 单次能力恢复**，并不让个体自主获知未知任务升级协议，也不支持无限任务演化或第三方服务授权；预置查询策略和已知图组合足以表示新v2规则，所以实验通过仅支持受控资源续期+反馈纠错。内核掌握原回执与任务fingerprint，不是公开可验证的密码学令牌；任务在新标签确认后再次变更仍有 TOCTOU 风险，许可管理与生产级认证尚未检验。G0 GO，G1/G2 NO-GO；E001 未预注册。

[S38 活动 VM](../../crates/organism-seeds/src/s38_task_recovery.rs) · [原生对照测试](../../crates/organism-seeds/tests/s38_task_recovery.rs) · [内核受费授权](../../crates/world-kernel/src/lib.rs) · [定向 CI](../../.github/workflows/s38-task-recovery.yml)。

## 失败历史
- [首轮 workflow #44702007308](https://github.com/iFrankStudio/OCEE/actions/runs/44702007308) 没有创建 Rust Job：workflow 名称包含未引用的冒号导致 YAML 解析失败；修订 [abf1e89](https://github.com/iFrankStudio/OCEE/commit/abf1e89f592821085e757d66b28fad8d6a9a7690) 只修正工作流语法。
- [初轮原生 Rust CI #44702131907](https://github.com/iFrankStudio/OCEE/actions/runs/44702131907) 编译成功但4项实验全部在 Program.validate 返回 `InvalidProgram("unbounded control-flow cycle")`，属于原生 VM 静态安全保护，不能当作学习结果。修订 [e2a3972](https://github.com/iFrankStudio/OCEE/commit/e2a3972309f08838ff980fbe762355318c156ef9) 采用独立最终验证区块，移除重复续期入口；未更改种子、目标规则或内核配额。

## 2026-11-20 最终原生验收

- **[S38 定向原生 CI #44702623338](https://github.com/iFrankStudio/OCEE/actions/runs/44702623338) completed/success，Rust 4 passed / 0 failed / 49.40 秒**；核验源码 SHA [`2383cf3`](https://github.com/iFrankStudio/OCEE/commit/2383cf30daa2143151e27f9aa0b71fbe618a66f8)。原生 VM 的4项测试分别执行：真实 v1→v2 规则改变后的恢复与 S37 对照、跨家族旧工具真实CAS修复、伪造/篡改回执不能购买额外查询、内核直接配额/版本授权约束。
- **原地真实任务演化**：P2 world seed55 在主动VM首次受费 Probe 52 A 并保存v1缓存后，研究者更改公共任务至v2并真实切换内核隐藏规则为 `2×Len`。活动VM第一次 VerifyProbe 见 Conflict，利用真正的旧回执请求 RenewProbe **7 A**，内核将额度绑定新的任务hash；随后 VM 选择 `[7,8,9]` 第二次 Probe **52 A**（新标签 **6**），通过真实受费 CAS 覆盖缓存 version0→1，再最终 VerifyProbe Ok，接受新的 **4指令 Len+Len** 程序，新任务未见输入的第四操作数真实变为 **10**，C/B实际返回正确、B气费 **24**。P2 总 **A26,653 / 260 ticks**，根hash `d15bb3aa1985671fe18c881647c3a04e089323a569c00171837df83c390106f2`。同样任务变化的旧S37控制 **A8,359 / B0**，没有伪B。
- **来源 P1 v1 程序实际跨家族迁入 P2 v2 并更换**：source P1 world seed1 不使用研究者预装答案而在正常v1任务中自主学得工具 hash `ccbec864ef8a002aa15e07a541cfca57200fa8770f51020274f1b1d359aa43c5`（P1 **A15,096**）。通过真实两次WRITE付费 **import29 A** 导入target P2 world55；P2第一次合法Probe后研究者切换v2，目标通过第一次Verify Conflict→正常Renew→第二次Probe+缓存CAS，独立拒绝旧P1工具，重新搜索得到不同程序 hash `39c1196f961fc3538686460216d0b0037ed950ec03ac6c5aa5add54addbcf098`，受费 CAS **registry version0→version1**，最后真实C/B FOLD32成功（P2 **A26,945 / 267 ticks / B24**，P1 B也24）。目标没有被宿主直接替换程序。
- **污染证据不能授权**：自有存储预先放置假标签而没有内核签发记录，正常 S37 VerifyProbe 即 NotFound，**没有RenewProbe或新Query，B0**。另一场景先正常Probe/缓存及第一次真正挑战EXEC，再由研究者对缓存进行真实收费 CAS 篡改，随后改任务为v2：第二候选实际READ到假值，Verify Conflict 后的 RenewProbe 因回执字节与内核签发记录不符而 **Conflict（仍收7 A）**，不批准第二次查询、B0。内核 `probe_renewed_for` 始终为None。
- **内核授权完整约束**：v1期间 RenewProbe Denied，不改额度；v2错误旧回执 Conflict并付7 A，仍不能额外Probe（Quota）；提供真实旧回执后合法 RenewProbe Ok/7 A，v2第二次 Probe Ok/52 A并返回 v2标签6，计数升至2；旧签发回执 Verify Conflict、新回执 Verify Ok；重复RenewProbe与第三次 Probe 均Quota，不继续收费；snapshot重载保持内核状态hash一致。

### 原生失败与严格科学边界
- [CI #44702268077](https://github.com/iFrankStudio/OCEE/actions/runs/44702268077) **2项成功、2项测试断言失败**。第一处误将 P1 世界的真实公开 `transform.held_out_input` 视为手写固定 `[4,5,6,7,8]`，导致已正确生成的实际第四数 **50** 与错误参考期待 **13** 不符；修复为读取P1自身真实 heldout 输入再计算oracle。第二处在初次Probe回执已经被VM continuation真实持有时才修改磁盘缓存：此时VM合法地使用真实**在途**回执获续期，但后续 CAS 发现缓存version冲突并阻止错误覆盖；这不支持“假缓存获授权”结论。修订 [`2383cf3`](https://github.com/iFrankStudio/OCEE/commit/2383cf30daa2143151e27f9aa0b71fbe618a66f8) 将篡改时机移至首次第三标签实际EXEC后、下一候选READ前，才真实验证污染缓存Renew被拒。保持种子、内核策略、任务规则与预先定义的成败条件未变，所有失败 CI 保留。
- [CI #44702131907](https://github.com/iFrankStudio/OCEE/actions/runs/44702131907) 的VM静态安全环检测及 [#44702007308](https://github.com/iFrankStudio/OCEE/actions/runs/44702007308) 的workflow YAML错误同样保留，不能暗示首次提交即成功。
- **此版本仅恢复研究者授权的固定 v1→v2、一次且仅一次新增探测额度**，并非获得未知服务的合法授权、开放式语法发现、多轮连续自主适应或互联网接入；内核受信且知道任务真实规则，认证是内核回执等值校验而非外部密码学签名。假设只两个公开训练标签、任务新规则恰可由原有54候选表达，实验样本仅 P1/P2 个别种子及对照，无确认性统计。**G0 GO；G1/G2 NO-GO，E001未预注册。** 对应代码完整 workspace [#44702623319](https://github.com/iFrankStudio/OCEE/actions/runs/44702623319) 尚需独立确认，不能把定向4/4冒充全量验证。

