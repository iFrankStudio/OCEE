# P2 实际 EXEC 反馈驱动的可用性路由（开发性候选）

状态：代码已提交，原生 Rust 两臂尚待实际运行。此实验不构成 E001 正式预注册、净效用学习或开放式演化证据。

## 动机与已知负例

上一个同起点等能力付费 L/R 研究（原生 #44995228916）获得本地完整任务 A=12236、B=0、C=0；远程完整任务 A=47359、B=24、C SEND=61；两臂均返回真实成功。但分叉后一字节 L/R 由研究者写入，根程序没有自主作路由选择。新切片检验更狭窄、可证伪的命题：在已经拥有研究者编制的本地算法与 P2 专家的前提下，同一个活动 VM 能否只根据自己实际执行本地工具的反馈，选择本地完成或明确降级使用远程服务，而非读取研究者的路由字节。

## 前置固定设计

- world seed55，P2 协议开发 seed2027、C 延迟8、A 真实收费 genesis 12000 gas/epoch、14k 自主 tick 上限；维持原 S45 root→P1→P2 公共学习、任务与 FOLD32 校验。
- 在 P2 handover 处，由同一 VM 始终先 Read 自己生成的 16 字节数据，再真实 Exec 固定 hash 的本地泛化 FOLD32 纯 VM 工具。只有真实 Exec 返回 Ok 才保存本地结果并成功退出（B0/C0）。只有真实 Exec 返回 NotFound 才付费切换现有 P2 远程专家，走原有真 C/B、u64 ACK/RESULT 因果确认。Gas/InvalidCode/Denied 等不能被当成 NotFound 自动转远端。根据首次真实 Gas 失败，新增限 8 次 RepeatBounded 的本地 Exec Gas-only 重试，重读同一实际四数输入并跨真实 epoch 继续；仍没有资源赠送、延长观察期或改写任务。其它反馈 fail-closed。
- 两个明示的研究者初始能力条件：PRESENT 在 genesis 时真实付费安装本地工具；ABSENT 不安装这个本地程序，绝不通过研究者改写路由数据。二者的初始状态与收费不同，不能把其总 A 成本差误报为等起点纯因果净效益。共同 root / P1 / P2 程序相同，路由决策在 VM 的实际反馈图内。
- 预先固定成功判据：PRESENT 只有 1 次真实本地 Exec Ok、B0、C0、正确落盘结果 U32(1)；ABSENT 有真实一次 Exec NotFound、真实 RemoteEval B24 仅一次、真实 C Send、受信 ACK 和 RESULT u64 ConfirmResponseFor、同值正确落盘 U32(1)。双方 snapshot 恢复哈希相等、Invalid/Denied 0，无不合法 RestartRoot。
- Rust 运行独立 matrix 两臂，日志 P2_FEEDBACK_ROUTE 包括真实 genesis/VM/动作 gas、C/B 账本、NotFound/Ok 计数、真正 RemoteEval 请求 ID、完成 step、fold32；CI 即使绿色，也必须逐臂检查日志中 success=true 且对应分支为真正 B0/B24。

## 解释边界与下一步

这里的行为选择确实发生在活动 VM 内，观察来自合法、实付世界动作，而不是由研究者写一个路线字节；但可用性回退策略、两个求解器和验证目标仍完全由研究者提供。它**不测量两路线的长期收益，不学习最优成本策略**；它甚至不能在一个形式合法但输出错误的本地工具上识别语义损坏。只有 NotFound 才允许切换，暂不修复 Gas 资源等待或跨 episode 价值记忆。对不同初始化能力条件的成本数据只作描述，不作等价 baseline 推论。

下一步若本窄CI实际通过：测量真正内部收入/成本反馈并在多个任务生命周期中付费保存、校验、继承自己的路径表现，增加错误但成功返回的本地工具语义反例，再与冻结的研究者固定 L/R 基线同任务比较。无后验抬预算、挑选 seed、丢弃失败；保留 2026-11-22 旧 L/R 负面研究结论。科学闸门：G0 GO；G1/G2 NO-GO；正式 E001 未校准/预注册。

## 2026-11-22 — 原生初次红例、因果故障与固定修复

- [第一次两臂 Run #44997491803](https://github.com/iFrankStudio/OCEE/actions/runs/44997491803) @ 84c3f7f：ABSENT 真实1/1通过，实测 Exec NotFound=1、真实 B24、A47017、C SEND61、u64远程与RESULT确认、结果1415179467；PRESENT失败，根在step1025返回U32(0)、无成功Exec/B0/C0。整个matrix为failure，保留原失败，不称实验通过。
- [第一次PRESENT末段诊断 Run #44997843628](https://github.com/iFrankStudio/OCEE/actions/runs/44997843628) @ 9e02d6a：同样在1025失败，最后42个tick大部分因Gas不足没有动作，说明失败根已经在上一资源周期进入回退分支；还不足以确定动作根因。
- [第二次只看最后实际动作 Run #44998024761](https://github.com/iFrankStudio/OCEE/actions/runs/44998024761) @ 9cef866：根因明确，第40步Read真实 remote-values-le成功，第41步实际 Exec 本地程序hash c600ba0f...，本次仅支付85 A gas、返回 Code::Gas；原根直接将非Ok且非NotFound当作失败，进入failed回传，而不是留存可恢复的预算不足状态。长时间Gas拒绝后第1024步VM支出33，返回失败U32(0)。此前“根本没执行本地工具”的表述应更正为“实际尝试并收费，但一次完整Exec没成功”；不得把Gas理解为本地能力缺失。
- 根据上述原生事实，只对本地Exec的真实Gas分支新增总8次受VM验证的 RepeatBounded 重读+重新Exec，NotFound仍唯一远程触发，非法反馈fail-closed；不改变协议seed2027、12k/epoch、14k tick、初始工具能力或任务，不补偿旧失败扣费。后置两臂须在新的完整Root hash下同时重验，**本段写入时原生回归尚待结果**。
