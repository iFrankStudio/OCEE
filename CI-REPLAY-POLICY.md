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
