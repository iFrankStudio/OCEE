# 单一活动轨迹的真实收费初始装载与 4,000 gas/epoch 降额组合

**状态：原生首次测试已完成，单条组合轨迹真实通过。仍属开发集工程实验，不是正式 E001 预注册。**

## 证据依据与确切缺口

截至 `9e811559949d28c96897147bbec9bca713acfe4a`，P2 个体已有两项互不等价的证据：

- [资源压力原生2/2 #44987651536](https://github.com/iFrankStudio/OCEE/actions/runs/44987651536)：协议seed587在已预装研究者程序、活动前**特权补满**到A 12000额度后，step8192真实遭遇降额4000，step8217正确结束、B24一次；它不包含有偿初始化时的初始缺额。
- [有偿初始化原生1/1 #44988134196](https://github.com/iFrankStudio/OCEE/actions/runs/44988134196)：协议seed2027的P1/P2/root由真实Write扣A 8818，自主活动前余3182，最终step3351真实成功；但是未到step8192，不知道真正经历了降额后是否还可完成。

不能拼接两条不同轨迹的成功来得出同时具备这两个性质的结论。

## 执行前固定

固定已见开发协议seed **587**（原有压力案例中最晚完成的高成本实例）、World seed55、P2家族、全程消息delay8、原S45 Phase0/Phase2 双界搜索程序与专用研究者工作任务及观测许可。只重构测试的共用启动夹具，不改任何活动程序、VM、世界服务/费用或协议生成器。初始资源 `a_gas_per_epoch=12000`，由真实初始四次 `Action::Write` 支付研究者提供程序的装载，总额预期8818、余额3182；不得再由研究者重置余额。世界 `ScheduledEvent::ResourceChange(A,4000)` 在逻辑step8192生效，后续按内核epoch补充。最多14000个自主tick，保留原有限上限，不因失败加长观察期、提高资源或替换seed。另一个seed2027既有成功用例维持原名称与独立历史通过记录。

主成功判据（新Rust单项测试强制）：**真实到达并越过8192步，最后A gas_per_epoch确为4000，而且在降额实施后审计日志中有非零、实际扣费的VM计算进展**；活动VM在有限14k tick内结束并返回U32(1)，真实B24仅一次，真实REMOTE_EVAL SEND u64 ID唯一、ACK与RESULT经`ConfirmResponseFor`认证、最终结果与独立operand`fold32`一致，世界快照hash一致，无Invalid/Denied或无因Root重启。保留`gas_refusals`、`VM/action gas`、实际完成step，以及新增机器行`P2_CHARGED_DOWNSHIFT`。

如果没有穿过资源变化或没有真实进展，即使任务偶然提前成功，也不算本研究目标成功。若任何判据失败，必须保留原CI红例，不能放宽断言取得绿色。拒绝将Gas拒绝等同于死亡、将Unknown等同于未执行请求。

## 限制

这是**研究者编制并安装的完整S45专用P2求解程序**，只验证已装载个体的计算进度保持与宿主资源约束。它不等于自行生成程序、能够长期维持多个不同生命周期、跨协议家族迁移或相对本地计算节约A资源，也没有E001标准四组对照。科学门槛仍G0 GO、G1/G2 NO-GO；正式E001尚未校准或预注册。

## 2026-11-22 — 首次真实原生验收：收费初始化与4k降额同轨迹成功

**[GitHub Actions Run #44989858138](https://github.com/iFrankStudio/OCEE/actions/runs/44989858138)**，源码 SHA `11f5845cb7f76ff3b43e7f7b31248a24e61291d0`，workflow/job `completed/success`。目标原生 Rust `p2_charged_genesis_crosses_real_4k_downshift_with_b_once`：**1 passed、0 failed，耗时564.36秒**；唯一被测实例保持此前已见的开发 P2 seed587、世界seed55、全程C消息delay8、原S45程序（未更改算法）、不超过14000个自主tick。

原生`P2_CHARGED_GENESIS`与`P2_CHARGED_DOWNSHIFT`两条独立审计行的完整关键结果：

| 项目 | 真实世界执行记录 |
|---|---|
| 初始资源 | A: `12000 gas/epoch` |
| 四次真实genesis Write实际扣费 | **8818 A gas**，首次自主tick时只余**3182**，没有特权补满 |
| 降额事件 | step8192正规世界事件，从12000降至**4000 A gas/epoch** |
| 降额后实际活动 | **267个带非零VM gas的tick**；VM gas真实扣费合计**9911** |
| 最终状态 | `finished=true, return=Some(U32(1))`，step **10303**，终态A限额仍为4000 |
| B真实执行 | **24 B gas一次**，`REMOTE_EVAL`唯一u64 SEND ID `206` |
| C真实请求确认 | `confirmed_remote=true`；`confirmed_result=true`，RESULT候选ID `96, 304`（其中合法RESULT收到确认） |
| 活动VM A消耗 | **94229 gas** |
| 活动world动作A消耗 | **3105 gas** |
| 初始+活动真实总A消耗 | **8818 + 94229 + 3105 = 106152 gas**，由真实epoch grants支付 |
| GAS资源拒绝 / 错误 | **7500次Gas拒绝**；`invalid=0` |
| 初始活动图代码内容哈希 | `0f9163bdff40a2ac5cbee63586353ea394f639d6dc7cc190ac34c811634c518e` |

硬判据通过：实际跨越8192，最终额度4000，且降额后**真实VM gas消耗>0**，不是仅更改配置；S45程序顺利返回U32(1)，B24恰好一次、REMOTE_EVAL唯一SEND、ACK/RESULT按对应u64 Request ID `ConfirmResponseFor` Ok，落盘结果独立`fold32`核验，world snapshot恢复与hash一致、没有无因Root重启/错误执行。

### 对照解释与边界

此前同一个seed587的[免费重置genesis压力实验 #44987651536](https://github.com/iFrankStudio/OCEE/actions/runs/44987651536)在step8217真实成功，原活动VM94045、动作3097、Gas拒绝5419次。当前完整收费genesis在step10303完成，额外约2086步和2081次Gas拒绝；计入初始化后总A成本106152。**不能**把资源等待等同于自适应决策：它反映有状态、可恢复的VM继续执行以及世界按epoch发放算力，资源分配/探索算法仍来自研究者。

只验证一次任务，未证明永久存活、跨任务/跨家族自适应策略、经验保存并复用、自主设计新程序或相比本地计算的净效益，也未校准正式E001的完整S/A/B/C/D四组条件。此新样本不是未见盲测（seed587在此前开发回归已使用）。**科学门槛仍G0 GO、G1/G2 NO-GO，E001未校准或正式预注册。**

定向workflow临时push触发在结果归档时移除，仅保留`workflow_dispatch`手动重放，避免对其他研究提交重复运行约9分钟的昂贵实验。
