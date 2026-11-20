# OCEE S41 — 受费早期故障检测与分区生命周期恢复（待原生验证）

**2026-11-20 · 开发性假设/工程候选。已提交源码，原生 CI 因运行器未分配尚无法验证。保持 G0 GO、G1/G2 NO-GO、正式E001未预注册。**

## 继承的可复核基线
- [S40 4/4 原生成功 #44710108675](https://github.com/iFrankStudio/OCEE/actions/runs/44710108675)：固定P2 seed55世界中，真实4数REMOTE_EVAL第一次入队后，任务v1→v2变化导致内核丢弃旧request id26（B0）；活动P2专家等待128个空接收轮次后才询问FenceStatus，并主动请求一次RestartRoot。S38根通过真正许可/Probe/CAS重学及S39双重任务栅栏发送新请求，**第二次消息B24一次**。跨家族来源也通过。开发证明不等于开放式演化。
- [同版本S40全workspace #44710108719](https://github.com/iFrankStudio/OCEE/actions/runs/44710108719) 最终是 **completed/cancelled**，不是全量通过；其状态已保存在工程检查点。

## 新增机制与可证伪假设
**H41a（延迟）**：个体不必等到S40固定的128次Receive重试结束。相同协议种子/P2服务/可重复消息环境中，S41 P2专家遇到任何真实 `Receive→Code::Empty` 时，沿内置的有限分支调用一次 `WorldCall::FenceStatus()`；仅当内核对该actor确实存在一个S39服务消费时留下的 `TaskFenceDrop`，才使用继承的S40 `RestartRoot` 意图。否则返回原来的 `s23_receive_retry` 有界循环，不能因普通未收回复擅自恢复。**比较指标必须是从真正的drop.step到真正的RestartRoot审计step的间隔**；S41必须显著小于同等条件S40，且两者新请求都实际B24一次。机制是**活动VM在空接收后提早轮询**，不是内核主动push通知，不能混淆。

**H41b（有限计费）**：原S40的 `FenceStatus` 在返回Empty时不消耗A资源，可能被任意空轮询浪费世界tick。S41令该动作无论返回Ok还是Empty均**收费1 A gas**，在审计记录中可见；仍由原有128有界循环限制后续轮询次数，真实重启仍只许一次且收费8 A。正常世界必须B24、不发生任何自动RestartRoot，不能出现无故B丢弃或无界重试。

**H41c（不再依赖逐对象白名单）**：原S40 `RestartRoot` 固定删除5个特定的self路径，实际新代码若另产出某个结果对象可能在第二轮造成CAS冲突。S41改用前缀生命周期合同：对于当前actor拥有的对象，`/self/results/`、`/self/work/`、`/self/discoveries/` 下所有键一并视为**本回合派生状态**并在受费RestartRoot时删除、正确扣减storage_used；`/self/evidence/`、`/self/tools/`、`/self/versions/`、`/self/active`、`/self/notes/`以及相似但非真前缀的`/self/workbench/`必须保留。真实测试在上轮运行前新增**从未列入S40白名单**的三个派生对象，以验证泛化性并检查快照hash。此合同仍由**研究者固定给出三种前缀**，不是个体自己推断状态因果依赖；真实生态中可能误删被放错命名空间的重要数据。

## 冻结的原生测试规格（尚无通过结果）
1. **相同P2 seed55真实v1→v2任务迁移**：S41与S40独立世界都真实Send四操作数消息后，在C/B消费前更改任务，使旧消息被丢弃且B0。运行到真实恢复并产生新B24；读取两者审计 `drop.step` 与 `RestartRoot.step`，要求S41至少比S40少40 ticks；比较不同实际新消息ID、v1第四数48/v2第四数10、服务事件只一项B24、探测次数2、缓存和注册CAS各从v0到v1、active version3。新增三种未知的派生对象均被删、三个持久对象仍保留、storage_used匹配真实对象长度和；额外300 tick不重复B计费。
2. **P1旧程序向P2迁移**：冻结root字节在source P1 seed1和target P2 seed55一致；目标受费导入真实P1 v1程序，真实发送后遭v2漂移并产生S39 TaskFenceDrop；S41目标尽早主动请求重启、S38再次查询并CAS修复旧代码、S39重新Arm新任务hash，目标B24一次，来源正常B24。
3. **无丢弃的负对照**：直接`FenceStatus`返回Empty却真实计费1 A，直接`RestartRoot`必须Denied；正常世界P2继续完整协议并正常B24，活动VM不得自行发起成功重启，计费和快照可复核。
4. **历史丢弃但当前任务回退**：实际旧请求因v2变化被S39丢弃后，再由研究者使任务字节恢复旧指纹。个体即使看到历史丢弃记录，也不能绕过内核对当前指纹的检查而RestartRoot；无新查询/新B假成功。这个对照不声称已授权的回滚可执行旧请求。

## GitHub Actions 运行器阻断与透明状态
- 首次提交包含S41定向、S40原生回归和全 workspace，各个独立workflow均在约3秒内 **completed/failure**，但 GitHub Job API 的 `steps=[]`、`runner_id=0`，即**没有分配GitHub-hosted runner，根本没有执行Checkout、Rust编译或测试**： [S41 #44748358687](https://github.com/iFrankStudio/OCEE/actions/runs/44748358687)、[S40 #44748358559](https://github.com/iFrankStudio/OCEE/actions/runs/44748358559)、[全量 #44748358715](https://github.com/iFrankStudio/OCEE/actions/runs/44748358715)。
- 已通过连接的GitHub API对S41原作业独立发起一次重试：`run_attempt=2` 同样在约2秒内失败，`steps=[]`、`runner_id=0`，日志URL不存在；因此不是已发生Rust编译失败，**也没有实验数据支持S41已通过**。存在私人仓库Actions计费/配额、组织运行器政策、GitHub托管运行器受限等可能，当前连接的API不足以确认根因。需从GitHub Actions运行详情和组织Actions/账单页面确认，恢复运行条件后手动一次运行 S41、S40定向，再视需要运行 workspace。避免每次push重复触发所有历史study耗尽额度。
- 在无 `cargo`/`rustc`、无外网可下载Rust的本地执行环境中无法替代原生Rust验证。已完成的仅是程序图路径、WorldCall参数类型、状态前缀、测试目标的静态代码复核；最后有一处Rust前缀引用和`&mut String`比较被改成显式解引用/`as_str()`。修复提交带`[skip ci]`，不为无法启动的runner继续制造无意义的workflow。

## 科学界限与后续方向
S41尚处于**开发代码待原生验收**，不能计入任何新研究突破。即使未来四项成功，也只是研究者预设有限循环、固定task v1→v2、可信内核签发、P1/P2专家、有限54候选及三个**人工规定生命周期前缀**下的一次局部恢复优化；没有实现通用异步订阅、未知数字服务自动恢复、任意记忆因果分类、现实网络分布式exactly-once，也不能证明G1/G2或开放式演化。下一实验S42应降低`actor`级丢弃状态歧义（绑定具体请求ID），并用声明式来源依赖/资源权利替代按目录删除，而不是增加更多内核特例。

[S41 活动专家源码](../../crates/organism-seeds/src/s41_early_drop.rs) · [4项预定原生对照](../../crates/organism-seeds/tests/s41_early_drop.rs) · [世界内核生命周期和收费规则](../../crates/world-kernel/src/lib.rs) · [S41 定向CI](../../.github/workflows/s41-early-drop.yml)。