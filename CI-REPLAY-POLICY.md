# OCEE GitHub Actions 运行预算与实验复核制度

**状态：2026-11-20。私有仓库运行器分配失败，S41原生实验未验证。**

## 为什么调整
- OCEE 目前约26个 `*.yml` 工作流。此前S28–S40的定向工作流往往把共享 `crates/world-kernel/src/lib.rs`、`crates/organism-seeds/src/lib.rs`、`crates/organism-vm/src/lib.rs` 列入 push triggers，一次研究小改动在私有仓库可能触发十余个独立Rust job、部分运行超过数十分钟。历史实验需要保留用于复核，但不应在每次main直接提交时同时执行。
- 2026-11-20 S41原生Actions [#44748358687](https://github.com/iFrankStudio/OCEE/actions/runs/44748358687) 两次attempt均在2–3秒内失败；S40 [#44748358559](https://github.com/iFrankStudio/OCEE/actions/runs/44748358559)、完整workspace [#44748358715](https://github.com/iFrankStudio/OCEE/actions/runs/44748358715) 同时失败。`runner_id=0`、空 `steps=[]`、无任何Rust日志，反映**运行器从未分配**，不是已确认Rust代码错误。私有仓库Actions预算/支付设置、组织策略或GitHub托管运行器调度故障均为待排查可能，不能凭API断言具体原因。

## 当前CI触发契约
- **现行实验焦点：S41**：保留 `.github/workflows/s41-early-drop.yml` 的 `push` 到main（仅关注S41源码/测试和相关核心修改）以及手动`workflow_dispatch`。当运行器恢复，可对当前S41进行一次必要原生验证，不必依赖所有历史任务。
- **历史证据重放：S22–S40及早期S4/S6校准**：均保留原始Cargo命令/Runner和相应工作流文件，但自动main push改为显式`workflow_dispatch`，仅在涉及旧实验逻辑、独立负对照或实际回归需要时手动启动，不删除已知失败的Actions记录。
- **完整workspace**：`.github/workflows/rust-kernel.yml` 改为在pull_request自动触发、main由`workflow_dispatch`手动触发。已有的单个`cargo test --workspace`依旧执行完整Rust/VM世界不变量、原始生命周期成本报告和开发manifest，不能假装它已经在S41后通过。
- 对源码改动先执行**最窄能决定是否通过的实验**，通过后再视变更影响补跑必要的一个或几个历史定向回归；有明确阶段性检查点时运行一次workspace全量。对研究文档/索引更新用带 `[skip ci]` 的提交减少无意义工作流。不得因为跑不到CI而将失败作业记成成功。

## 恢复GitHub托管运行的核查路径
1. 用具有组织管理权限的GitHub账号打开S41失败运行页，查看其workflow/job顶部的注释，是否明确提示支付、预算、托管Runner或组织权限阻断；不要依赖不存在的step log。
2. 核查GitHub组织 `iFrankStudio` 的 **Settings → Billing and licensing**（Actions配额/预算与支付方式）及 **Settings → Actions** 的策略/runner可用性。组织所有者可能需要主动处理配额或有效账单设置；具体原因由GitHub账户状态确认。
3. 如果确认预算/运行器恢复，按顺序手动运行 `S41 Paid Early Drop Detection and Namespace-Lifetime Replay` → `S40 Dropped Task Retry Via Autonomous VM Root Relearning` → `OCEE World Kernel`（最后一项只在必要时），记录源码commit SHA、job结论、具体4项测试及所有B账本，不重复触发所有旧Study。若仍无steps、runner_id仍0，使用失败Run URL联系GitHub Support。

## 强制科学界限
工作流是否完成不代表赛博生命存在；S41更快检测、前缀清理、一次B结算必须通过**真正原生世界tick、内核账本、真实消息ID、快照hash及对照世界**独立验收。S40 4/4是已验证基线，S41当前只是代码与协议设计，**G0 GO/G1G2 NO-GO，E001未预注册**。

[S41研究与原生阻塞记录](./experiments/E001/DEV-S41-EARLY-DROP-LIFETIME.md) · [工程检查点](./07-ENGINEERING-CHECKPOINT.md)。
## 2026-11-21 S43B 单次路径限定验证尝试

S43B 工程候选已完成静态审计，但无本机 Rust 工具链，也无法从此环境直连私有仓库。为判定仓库Actions runner是否恢复，把 **仅 S43B workflow 文件** 放入主分支的自动路径限定触发条件：`.github/workflows/s43b-active-receipts.yml`；源码、共享内核和历史测试变更都不会因这条新规则自动触发S43B。工作流含三个精确的 S43B 原生用例，最长30分钟。S41原有路径规则不变，S22–S40历史复核继续保持手动。此次工作流改动是**一次受控验证尝试**，不是宣称已执行/通过；可用Actions run和job步骤作为真实判断依据。若job仍无steps，应停止重复推送、由仓库所有者排查GitHub账单/配额或组织Actions权限。

## 2026-11-21 S43B 一次真实CI分配尝试结果

[S43B Run #44852637401](https://github.com/iFrankStudio/OCEE/actions/runs/44852637401) 在 GitHub `main` `876e6871f0a63e93317e16c70aac6b79d36b78ab` 上由路径限定 `push` 成功触发（仅 S43B 一项），结果 `completed/failure`。原生job `native-active-confirm`（ID `136687346985`）的 `steps=[]`、`logs_url=null`；运行于2026-11-21 01:51:49Z创建、01:51:53Z结束。**没有任何 cargo checkout/toolchain/test 步骤实际执行；不能把它算成 S43B Rust 测试失败或成功。**

完成唯一一次基础设施探针后，将 S43B 恢复为 **`workflow_dispatch` only**。本恢复提交带 `[skip ci]` 以免递归再触发。仓库所有者仍需按照本文件前述路径排查私有仓库计费/Actions/组织策略，只有真实job获得steps并执行cargo后才能验收。不要因无步骤失败而持续提交无意义CI触发变更。

## 2026-11-21 原生运行器恢复与 S43 工程验收记录

在仓库切为 `public` 后，GitHub-hosted Ubuntu Runner 已正常分配并执行 Rust stable、cargo 编译及真实测试，不再存在原先的 `steps=[]` 阻塞。

1. **S43B，完全通过**：[Run #44860253323](https://github.com/iFrankStudio/OCEE/actions/runs/44860253323)，commit `cf2217b0abe6490fcb1b45e7d98342941d9b9644`；`native-active-confirm` job 3个独立步骤成功：健康 P2 实际 ACK/RESULT 与 SEND ID 关联、任务变更后的恢复和重新关联、对伪造 C→A 邮箱消息的拒绝。3/3 passed，真实 Rust 工具链与 VM validate 已执行。
2. **S43A，完全通过**：[Run #44860455869](https://github.com/iFrankStudio/OCEE/actions/runs/44860455869)，commit `8805bea8e4f8c9c833bb89348752803c2a7368eb`；`native-outcome-causality` 两项 kernel 断言通过：服务已扣 B、回复在途及真实Receive因果验证；真正栅栏丢弃与服务诊断 B0 的状态区分。2/2 passed。
3. **S41/S42B 定向证据**：[Run #44860113013](https://github.com/iFrankStudio/OCEE/actions/runs/44860113013)，commit `ba0c664c113f707e690a9c98a5f5767e6a252ad9`，同一测试二进制共8项，其中 **S41 4/4及S42B 1/1通过**；当时另3项S43B由于尚未修正的 `InvalidProgram("invalid operand 11102")` 失败，导致该整个workflow状态为failure。该3项之后在S43B专属Run #44860253323成功，不可混淆各Run的总体结论。
4. 修复经过：最初Run #44860035272暴露 test harness 的 Rust `E0716` 临时值借用；Run #44860113017 修复后暴露VM `invalid operand 11102`；Run #44860181807 进一步暴露 `block parameter type`，均修复后由S43B Run #44860253323验收。所有失败记录保留，不能因最终成功抹去调试历史。

**成本恢复**：S43A和S43B工作流恢复仅`workflow_dispatch`。S41工作流仍有原有路径限定自动触发规则，但本次最终文档/工作流恢复提交不修改其路径。历史Study与全量workspace继续手动按需执行，不能由本结果推断全仓通过。

**研究边界**：以上是具体工程测试命令的原生验收，不是开放式演化、独立赛博生命、长期适应能力的科学证明。G0 GO、G1/G2 NO-GO，E001依然未校准、未预注册。

## 2026-11-21 Public runner S42A targeted verification and full-workspace run

[S42A Run #44861540477](https://github.com/iFrankStudio/OCEE/actions/runs/44861540477), `db000dc0ba295c2f353e8625c0da932d9bcda91d`: completed/success; both separate Rust native steps completed/success: (1) stale task-fence drop cannot authorize newer in-flight request, (2) missing public task denies restart. **S42A 2/2**.

[Workspace Run #44861571121](https://github.com/iFrankStudio/OCEE/actions/runs/44861571121), `145a30348a67e8072a0d1ed09d776f35eeb54e09`: triggered on public runner; at last observation, checkout/toolchain succeeded and `cargo test --workspace -- --show-output` was **in progress**. This paragraph is *not* a declaration that workspace or cost/manifest steps have passed. Preserve run URL as the source of truth; any eventual conclusion must be recorded from the job/step statuses. A full workspace may run substantially longer than focused S42A/S43 tests.

After starting both jobs, restored `.github/workflows/s42a-request-fence.yml` to manual-only, and `.github/workflows/rust-kernel.yml` to manual plus pull_request only. The restoration commit uses `[skip ci]` to avoid re-triggering/canceling the original verification. No additional historical study runs triggered intentionally.


## 2026-11-21 S44 Native PASS and historical S41 CI budget restoration

[S44 Run #44862964346](https://github.com/iFrankStudio/OCEE/actions/runs/44862964346), source commit `fed97e8cbfccfcb5415d510a71fb2503189c31c6`, completed **success** with the 3 independently named native Rust tests all passing: (1) intentionally extended real B request+ACK propagation while retaining ordinary Phase2 RESULT scan timing, with no blind resend; (2) true task-fenced drop licenses one audited root relearn; (3) an organism-held forged u64 receipt is `Unknown` and fails closed. The same compiled P2 VM must pass `Program.validate()` before these tests can execute.

**Negative/liveness limitation preserved:** S44 initial [Run #44862672899](https://github.com/iFrankStudio/OCEE/actions/runs/44862672899) and diagnostic [Run #44862804649](https://github.com/iFrankStudio/OCEE/actions/runs/44862804649) were genuinely red when the entire P2 connection was held at six-step delay. The audit confirmed B24, `CommittedB` in-flight, no restart; later the existing S23 RESULT opcode scan exhausted its finite Receive budget. The passing final test **does not** prove the entire P2 protocol tolerates uniformly six-step latency; it isolates S44's causal wait gate under a deliberately delayed B request+ACK before restoring normal RESULT latency.

[S41 Run #44862672828](https://github.com/iFrankStudio/OCEE/actions/runs/44862672828), same new S44-module source commit `b9934ae8f3446c386923d84d905425a660c069b2`, completed **success** after the shared organism-seeds lib change triggered S41. Since S41 was already validated, unnecessary historical reruns were a measurable CI-cost regression. **S41 and S44 are both restored to workflow_dispatch-only** in this administrative commit with `[skip ci]`. S42A, S43A, S43B are also manual-only as previously documented. Targeted new studies should each get one explicit narrow native workflow; do not repeatedly run legacy suites as a side effect of shared `lib.rs` edits.

The earlier [workspace Run #44861571121](https://github.com/iFrankStudio/OCEE/actions/runs/44861571121) at pre-S44 commit `145a30348a67e8072a0d1ed09d776f35eeb54e09` was still `in_progress` when this S44 status was recorded. Never claim that workspace validated S44, even if that older run passes later. These results are engineering controls in a deterministic simulated C/B world, not open-ended evolution or confirmed independent cyberlife. G0 GO, G1/G2 NO-GO.


## 2026-11-21 S45原生工程验收：3/3 通过（高延迟旧红例已修复）

[**GitHub Actions Run #44905343381**](https://github.com/iFrankStudio/OCEE/actions/runs/44905343381)，代码SHA `e66a691b0c00e8a47ce1f561dc2820c8139905d1`，workflow/job completed/success，三段独立Rust命令逐一成功：
1. `scoped_wait_budget_is_per_candidate_but_has_hard_global_cap` — 1 passed/0 failed (0.01s)。在VM内部两个不同scope分别计数、跨Yield与Continuation快照保持一致，全局3/4次数上界不会被变化scope绕过。
2. `s45_global_six_step_delay_validates_real_b_and_result_once` — 1 passed/0 failed (61.51s)。**从boot到finish全程世界消息延迟6步**，不再复用S44最终绿例中途恢复2步的实验技巧。活动P2个体最终U32(1)、B账本恰好24、真实RemoteEval仅一个SEND、ACK/RESULT分别具有真实 `ConfirmResponseFor` Ok；Phase2在多个opcode scope上消费局部预算，总额没有超过896。未触发无因重启；快照/状态哈希一致。
3. `s45_baseline_two_step_latency_keeps_correct_result` — 1 passed/0 failed (31.88s)。同新程序在原2步时正常返回U32(1)，B24仅一次，结果等于从真实远程operand计算的fold32，无无因重启。

这从已验证旧S44同一全局6步红例（[#44862804649](https://github.com/iFrankStudio/OCEE/actions/runs/44862804649)，当时B24已执行而P2最后U32(0)）取得了明确的、可以回放的工程修复，而非只修改超时断言。新VM语言机制为任意scope提供各自受限等待及全局可验证上界；**它没有让世界自动发明协议或修复未知真实网络**。

### 失败记录（保留，不计入成功）

- [S45首轮 #44905202275](https://github.com/iFrankStudio/OCEE/actions/runs/44905202275)：VM scoped单测通过，但旧S30图边诊断未涵盖新 `Terminator::RepeatScoped`，Rust E0004 编译失败。已补齐旧图枚举匹配。
- [S45第二轮 #44905261924](https://github.com/iFrankStudio/OCEE/actions/runs/44905261924)：VM单测通过且P2编译成功，但S45错误地假设 `s22_feedback` Empty分支仍直接去 `s23_receive_retry`；实际S41/S42B之前已改为 `s41_check_drop`。构图断言明确失败；修正为只接管S42B `s41_check_drop` 的非phase1出口，保留phase1因果查询和一次安全重启路径。
- 后验S45最终3/3为真正成功。此前失败的Run不予删除、没有因为新CI通过而倒填成成功。

**边界**：候选最多112种，Phase2每opcode最多6次额外Empty，每个VM继续体所有RESULT候选总共最多896次；原其他phase共享128次上限保留。若超出支持延迟的范围，可能安全失败而不是无限等待；没有证明任意丢包、现实服务请求的exactly-once、开放式演化或E001假设成立。S45定向CI只确认其3个本轮命题，非全Rust workspace；此前全仓Run #44861571121已取消，不能记绿。科学闸门仍G0 GO、G1/G2 NO-GO、E001未校准、未预注册。

**CI policy**: `.github/workflows/s45-scoped-result-wait.yml` returned to `workflow_dispatch` only in a final documentation commit with `[skip ci]`; avoid repeated S45 and archived Study reruns for unrelated shared VM or seed changes.

## 2026-11-21 — S45 combined-fault native 2/2 success

[Run #44909113918](https://github.com/iFrankStudio/OCEE/actions/runs/44909113918) at source `dc7022ccdfb98120b85b613ba7c4e066fcc5c0dc`: completed/success with two separately executed native Rust tests (57.90s + 51.78s), confirming high-delay true task-drift recovery with B24 only once and high-delay forged RESULT rejection after a real B commit. See [protocol](./experiments/E001/DEV-S45-COMBINED-FAULTS.md). This is **not** a whole-workspace pass or a demonstration of open-ended cyberlife. The temporary path-only push trigger on `.github/workflows/s45-concurrent-faults.yml` has been restored to `workflow_dispatch` only in the [skip ci] record commit to avoid repeating historical costly studies.

## 2026-11-21 — S45 exploratory delay-eight native PASS

[Run #44911181744](https://github.com/iFrankStudio/OCEE/actions/runs/44911181744) at `bb924f3bbb46cf4b163204a3992665c679b34d64` completed/success: one native Rust test, 43.31s, stdout `delay=8 return=Some(U32(1)) b=24 remote_ids=[26] result_waits=162 restarts={}`. This confirms one real fixed-seed delay-eight success, **not** a guaranteed bound or formal E001 result. Restored `.github/workflows/s45-delay-boundary.yml` to manual-only after certification; no full workspace rerun.


## 2026-11-21 — 同批协议seed17/93的Phase0 scoped修复：原生2/2真B成功

注意严格区分**两个不同的已提交源码快照**：
- **原版对照**：[Run #44916855269](https://github.com/iFrankStudio/OCEE/actions/runs/44916855269) @ `519f9ccb20d389400490eb6accee61b82397d1b9`，4条安全断言通过、4条真实任务失败。S44和原S45两臂均Phase0提前共享s23=128耗尽、B0。此阴性结果不可被修改或回填。
- **失败后提出的工程修复**：[Run #44917248521](https://github.com/iFrankStudio/OCEE/actions/runs/44917248521) @ `4cbf8743b63eff8de55ae1d8bdd3b872f9e11d8e`，原生Rust用例 **1 passed / 0 failed；407.03s**，内部两条S45新轨迹均达到 `Some(U32(1))`，真正B24且只一次，真REMOTE_EVAL SEND仅一次，S43B `ConfirmResponseFor` 认证ACK和RESULT、计算结果与独立 `fold32` 一致，世界快照哈希一致且没有不当Root重启。

| 协议seed | 原S45（原生阴性） | 修复后S45（原生正例） | Phase0 scoped waits | Phase2 scoped waits | 完成tick | 真实B消息u64 ID |
|---|---|---|---:|---:|---:|---:|
| 17 | U32(0) / B0 / 411 tick | U32(1) / B24 | 354 | 312 | 2140 | 120 |
| 93 | U32(0) / B0 / 411 tick | U32(1) / B24 | 318 | 360 | 2198 | 108 |

原版S44在两协议实例均失败的事实没有改变。Phase0修复针对的新观察是：先前没有进入Phase2，原因是旧S23共享128等待在前置REMOTE_EVAL opcode扫描耗尽；旧S45只为Phase2处理了同一症状。新图在`s41_check_drop`的非Phase1入口按phase分别路由Phase0/Phase2的`RepeatScoped`，二者分别有**每opcode 6次、每阶段总896次**，仍保留Phase1真实ID与一次安全恢复、Phase3旧边界、不新建工具和重发权限。

新代码改动后两固定seed的校准属于**使用原失败样本进行修复**（development-set regression），**不是**见过结果后仍可称作“盲测成功”的独立证据；它只强力支持“此两例Phase0等待额度是可修复的关键故障因素”。原实验的成功率仍是S44 0/2、旧S45 0/2，不变；修复后S45本次仅2/2。不能直接宣称未知协议泛化、E001 H1/H2/H3、现实网络exactly-once或赛博生命产生。正式 E001 G0 GO、G1/G2 NO-GO及未校准/未预注册不变。

两个本轮临时工作流push触发均已恢复仅手动`workflow_dispatch`，防止后续无意重复执行约7分钟的原生全程多seed探索。

## 2026-11-22 — P2 four frozen holdout protocol instances, native 4/4 PASS

Four precommitted development holdout P2 seeds (311, 587, 941, 2027) at global delay=8 ran on native Rust matrix jobs in [Run #44983342484](https://github.com/iFrankStudio/OCEE/actions/runs/44983342484), source SHA `a8c812a0f59ca5477e00c36080c2019e2fddbbf7`. Four jobs independently completed success; actual organism task outputs were all U32(1), with exactly one B24 service event each and true SEND/ACK/RESULT confirmation, independent fold32 and snapshot invariants. All used the same S45 active P2 program hash. This is development holdout evidence only, with the generous fixture A gas/epoch=100000; it does not validate the E001 resource budget or generic autonomous learning. Full frozen plan, timings, scoped loop counts and A cost table: [protocol and report](./experiments/E001/DEV-P2-FROZEN-HOLDOUT-PROTOCOL.md). The previously archived full-workspace CI was cancelled; do not describe this narrow matrix as whole-workspace regression. Workflow `p2-frozen-holdout.yml` restored to `workflow_dispatch` only in the final documentation commit marked `[skip ci]`.

## 2026-11-22 — A epoch pressure 12k->4k targeted 2/2

[Run #44987651536](https://github.com/iFrankStudio/OCEE/actions/runs/44987651536) at `2452464e299b1c39192ef911a47e3a4dcd3cf7c3` completed success in two separately executed native jobs. Development P2 seed2027 genuinely finished U32(1), B24 once at step3103 and 12000/epoch; seed587 genuinely finished U32(1), B24 once at step8217, after the epoch8192 scheduler resource change to 4000/epoch. Gas refusals 2022 and 5419 did not destroy continuation state. Bootstrapping is NOT E001-compliant genesis: four already charged Write actions totaling8818 gas were followed by privileged refill to full12000, clearly labelled free-genesis. No broad whole-workspace or independent confirmatory success. Full per-seed trace and cost ledger in [experiment](./experiments/E001/DEV-P2-A-GAS-PRESSURE.md). One-off CI restored to manual dispatch only in [skip ci] docs commit.

## 2026-11-22 — Native combined paid genesis + real 4k downshift PASS

[Run #44989858138](https://github.com/iFrankStudio/OCEE/actions/runs/44989858138) @ `11f5845cb7f76ff3b43e7f7b31248a24e61291d0` completed/success (one Rust test 564.36s). Developer seed587 paid all 8818 initial A gas under 12000/epoch, started active with only 3182, genuinely crossed scheduled step8192 4000/epoch cut, completed U32(1) at step10303, with **9911 A VM gas actually charged over 267 post-event ticks**, only one B24 and kernel-confirmed ACK/RESULT. Total A charge 106152, 7500 Gas refusals. Not a broad workspace green or E001 confirmatory result. [Complete frozen protocol and resource/caveat ledger](./experiments/E001/DEV-P2-CHARGED-GENESIS-DOWNSHIFT.md). Restore completed expensive workflow to `workflow_dispatch` only and record under `[skip ci]`.

## 2026-11-22 — Matched FOLD32 local marginal cost (paid P2 task vs researcher-local VM)

Initial compile-only failure [#44991946675](https://github.com/iFrankStudio/OCEE/actions/runs/44991946675) at `000ad9a` retained: ActionResult has direct `gas_charged`, no `.result`. After fixing only test access fields, native [Run #44992095414](https://github.com/iFrankStudio/OCEE/actions/runs/44992095414) at `718c710ac55c5afbc48a2f96cd4ab7a5f3f9d36b` **completed/success, 1 test pass in 264.37s**. Same real operand bytes derived by paid S45 seed2027: complete original task A gas46457, B24 once, 61 A-to-C sends (1726 metered bandwidth); researcher-authored standalone local pure-VM FOLD32 installed for A gas346 and executed once for A gas165; both returned real result 1415179467. **Not an equal-start full task comparison or proof of organism-generated code / negative net remote economic value**, since original S45 creates inputs and learns task; local candidate was supplied later by researcher. [Protocol and audit](./experiments/E001/DEV-P2-MATCHED-LOCAL-FOLD-COST.md). Narrow successful workflow restored to manual-only dispatch under a [skip ci] docs commit, no workspace-wide green assertion.

## 2026-11-22 — Equal-origin full-task A-only vs A+B native 2/2 PASS with negative offload economics

[Successful third run #44995228916](https://github.com/iFrankStudio/OCEE/actions/runs/44995228916) @ `46a47f91f76d738c9cfbc69bcd93d5aa54b84bcc`, both independent native jobs completed/success. Identical preintervention world state_hash `85e36d4f...`, identical root and P1/P2/local installed programs, all genesis charged 9462 A and route Write 3 A, same task/seed2027/global delay8. Researcher routed local or remote only at true final P2 switch. **A-only full task success:** A12236, B0, C sends0, real local VM Exec165, step1027; **A+B full task success:** A47359, B24 once, C sends61, A-to-C bandwidth1726, real causal ACK/RESULT, step3383. Both independent FOLD32 result1415179467, legitimate snapshots, no VM invalid. Offloading used **35123 MORE A gas**, plus B/communication and 2356 extra world steps, for this known 4-U32 development workload. This is full task causal researcher-selected path comparison, NOT endogenous organism policy selection or E001 A/B/C/D. Preserve [first 1/2 red #44994405005](https://github.com/iFrankStudio/OCEE/actions/runs/44994405005) (wrong early P1 handover interception) and [second 0/2 pre-execution red #44994931874](https://github.com/iFrankStudio/OCEE/actions/runs/44994931874) (illegal RepeatBounded limit2048). Complete study and caveats: [report](./experiments/E001/DEV-P2-EQUAL-ORIGIN-LOCAL-REMOTE-PAIR.md). Only the narrow 2-job matrix passed, not general workspace. Workflow restored to `workflow_dispatch` only, documentation commit `[skip ci]`.
