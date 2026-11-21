# OCEE S43B — 活动 P2 个体对 ACK/RESULT 的请求级因果验收（待原生验证）

**2026-11-21，已提交待证伪工程候选。没有 cargo/rustc；GitHub-hosted runner 未确认恢复；以下成功/拒绝均为测试断言而非真实实验结果。**

## S43A 与 S43B 的区别
S43A 的内核已经提供 `ConfirmResponseFor(request_id:u64, received_Message)`：只有历史成功 SEND→实际服务事件→真实 RECEIVE 交付的三者一致才授权。S43B 使 P2 活动 VM **自己调用**这个接口，而不是研究者事后观察内核日志。

继承 S42B 真实 SEND 的八字节 LE u64 回执。原 S42B 只保存 phase=1 REMOTE_EVAL 的ID；S43B 将同样的保存路径扩展至 phase=2 RESULT 候选请求。为每次 (phase, opcode) 生成独立的 `/self/work/s43b-message-<phase-hex><opcode-hex>` 工作对象；候选 RESULT opcode 循环中的每条成功 SEND 因此不会覆盖另一请求的证据，亦无需引入无限增长/覆盖写权限。phase 0/3 的发现性请求不引入额外持久化。失败重启时这些属于S41清理的 `/self/work/` 派生状态。

S23 的 `s23_feedback_check_ok` 成功路径在进入原协议字节解析前被拦截：针对 phase 1/2，活动 VM 付费读回对应的 u64 回执，从刚刚收到的 RECEIVE JSON 数组中提取第一条原始 Message JSON，调用 `WorldCall::ConfirmResponseFor`，**仅 Code::Ok** 时才回到旧的 `s23_feedback_payload`、TLV解析、ACK记录/RESULT存储路径。伪造、乱序或错请求响应不会因协议形状恰好符合而被接受；这一版本采取**fail-closed 终止**，不是丢弃陌生消息后无限等待。

## 保持 VM 的128区块上限
不修改全局 `MAX_BLOCKS=128`。复用原 S42B 的 SEND 检查和存储区块，以 `((phase-1)/2)==0` 在研究者规定的 phase 0/1/2/3 中精确选取 phase=1/2；只增加5个回复因果验收区块，并移除新程序中不可达的S40 actor级恢复3区块及S41旧状态回复1区块。原S40/S41/S42B程序版本完全保留。新S43B仍由 `Program.validate()` 与128限制判定是否合格，**尚无原生结果**。

## 关键原生对照（已提交，未运行）
1. 正常 P2 运行：真实REMOTE_EVAL B24一次；在 S22 接受ACK和RESULT之前，必须看到活动 VM 的正确 `ConfirmResponseFor`，其 `request_id` 与成功SEND取得的各自u64回执匹配；无非法验收和角色级隐式状态查询。
2. 真实任务漂移：旧消息在C被TaskFenceDrop且B0，活动VM仍用旧ID进行恢复；重新Probe/修复后新消息B24，新的 ACK/RESULT 各自被确认，绝不把旧drop当ACK。
3. 研究者注入未由服务生成的C→A邮箱伪回复：RECEIVE可把此消息送到个体，但S43A拒绝Confirm，个体不得继续写ACK/RESULT或默默重启；世界快照确定性检查。

## 未解决问题
- S43A信任单一仿真内核与最近4096条审计，不是外部网络加密证书。这里认证的是运输身份/交付记录，不意味着语义答案正确；继续依赖 S22 既有 TLV/状态码/结果验证。
- phase/opcode 键在相同运行中若重复 SEND 同一操作码可能冲突（本实验每个RESULT候选只尝试一次），是已知活性局限；不声称通用异步任务调度。
- 注入/网络断连/ACK已送但个体没收到等真实世界常见情况仍需S44处理 Unknown/CommittedB 下的安全等待与人工审计。这里禁止盲目B重放。
- 科学状态仍为 G0 GO、G1/G2 NO-GO；正式 E001 未校准、未预注册。S41→S43B均缺少本次原生CI验收，不得把候选视为发现赛博生命。

[活动个体实现](../../crates/organism-seeds/src/s43b_active_receipts.rs) · [3项真正的VM回归](../../crates/organism-seeds/tests/s41_early_drop.rs) · [S43A内核](./DEV-S43A-REQUEST-OUTCOME-AND-REPLY.md)

## S43A.2 依赖加固
S43B 的 `ConfirmResponseFor` 现在不仅依赖 SEND→服务事件→RECEIVE，还依赖服务事件固定的完整消息内容哈希和实际交付时序。研究者注入同响应ID但不同正文、即使真的被RECEIVE取走，也不能把它错误当作原始ACK/RESULT。缺少旧账本指纹的快照安全拒绝确认。该改动与S43B的原生验收一起等待运行器。

## 原生CI第一次尝试的实际结果（2026-11-21）

为排除“只因手动触发不便而没有执行”的可能，曾将本研究单独工作流限定为仅自身文件变化的`main` push，并提交 `876e687`。结果 [S43B Run #44852637401](https://github.com/iFrankStudio/OCEE/actions/runs/44852637401) 被GitHub接受并触发，但job于约4秒后 `failure`、`steps=[]`、无checkout/编译/cargo日志。这是运行器/工作流环境阶段的**未执行结果**，不是S43B三个验收测试的运行失败证据。已恢复S43B工作流为仅`workflow_dispatch`，保留失败记录作为环境阻塞证据。科学状态不变；不要据此推断生命表现。
