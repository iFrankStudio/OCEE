# OCEE S42A — 最新受保护请求的丢弃证据门（待原生验证）

**2026-11-20。已形成代码与可证伪反例测试；本环境没有 cargo/rustc，GitHub-hosted runner 尚未恢复，原生测试结果未知。**

## 问题与机制
S40/S41 的 `FenceStatus` 检查同一 actor 的任一历史丢弃，`RestartRoot` 使用历史 observed task hash。当第一条受保护请求真正丢弃、同 actor 的第二条新请求仍在途时，这可能误把旧故障归因给当前请求。任务指纹相同不代表消息 ID 相同。

S42A 在内核 `WorldState` 增加可快照重放的 `last_guarded_send: actor→message_id`，只在受 S39 任务栅栏保护的 C SEND 真正提交时更新。统一谓词 `has_latest_recoverable_drop` 要求 kernel-authored TaskFenceDrop 的 actor、**最新发送请求 ID**、observed task hash 都匹配，并且 expected hash 与 observed hash 不同。它同时保护 `FenceStatus` 和 `RestartRoot`。缺字段的旧快照采用空表，保守拒绝恢复；不改动 S41 状态轮询1 A、S40 重启8 A/最多一次以及 C/B 服务协议。

## 已提交但未运行的关键对照
内核单测在延时8 tick 的模拟 C 世界连续成功发送两个受保护请求，之后研究者改变公开任务。等第一条在服务阶段真的 TaskFenceDrop，但第二条仍排队时，必须看到 `FenceStatus=Empty`、`RestartRoot=Denied`，且 Empty 收费1 A。等第二条也因相同任务漂移丢弃后，才返回 `FenceStatus=Ok`；消息 ID、B0、完整世界快照均由代码断言。**这些是测试的预期断言，不是已经取得的运行结果。**

## 科学边界与下一步
此修复采用**最新成功受保护 SEND**作为内核保守近似，并不等同于 VM 显式持有其当前等待的 request ID。并发场景可能错拒合法恢复（优先安全，损失活性）；真正的 S42B 需要把 u64 请求回执沿 VM continuation 持久携带，区分 ACK/RESULT 的请求归属，并处理 B 已完成的不可回滚边界。派生对象仍靠研究者指定生命周期前缀清理。

**验证顺序**：恢复 runner 后手动 S42A 单项内核 → S41 4 场景 → S40 4 场景 → 必要时完整 workspace。当前 S41/S42A均不记科学突破；G0 GO、G1/G2 NO-GO；正式 E001 未校准、未预注册。

[内核及反例源码](../../crates/world-kernel/src/lib.rs) · [S41](./DEV-S41-EARLY-DROP-LIFETIME.md) · [CI制度](../../CI-REPLAY-POLICY.md)

## S42A.1 缺失任务授权边界（2026-11-20，待原生验证）

静态审计发现此前谓词将 `Option<String>` 的 `observed_hash==current` 直接比较：公开任务对象消失时两者可能同为 `None`，且 `None != Some(expected_hash)`，从而让真实丢弃记录错误通过恢复资格门。此处属于源码可证明的授权条件漏洞，尚无运行结果。

在 [修复提交 3ed6517](https://github.com/iFrankStudio/OCEE/commit/3ed65178695a94cb665a2989a5c1592a05f43159) 将当前任务缺失直接拒绝，并要求丢弃记录具有与当前任务相等的明确 `Some(hash)`；没有增加额度或放宽重启权限。增加**一项**最小真实发送→任务删除→服务丢弃反例：要求 `observed_hash=None`、`FenceStatus=Empty`且收费1 A、`RestartRoot=Denied`、B0、无重启、快照重载一致。**测试仅已提交，尚未运行，不可写成通过。**

后续先执行 `cargo test -p ocee-world-kernel s42_missing_public_task_never_authorizes_restart`，再执行原S42A最新请求反例，S41与S40定向回归。GitHub Actions runner 仍需确认；避免自动重复触发失败运行。正式E001未启动，G1/G2 NO-GO。
