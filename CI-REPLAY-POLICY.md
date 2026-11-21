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
