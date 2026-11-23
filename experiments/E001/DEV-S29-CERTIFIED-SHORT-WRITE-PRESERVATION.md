# DEV — S29 已认证引用索引对严格短无害写入的付费保留（研究前冻结）

**状态：预冻结协议；未获得本轮原生结果。** 基线 `5c6759167e0d3f7d2f9b654320c5709c282830c1`。上一轮[同写入不同时间的真实4/4报告](./DEV-S29-CROSS-EPOCH-CERTIFICATE-CHURN-PROTOCOL.md) @e12f53c9：BEFORE_INDEX外部1B Write3A，维护VM+世界Action合计1887A；AFTER_INDEX同内容同目标但在已收费Index之后写入，保守废掉索引，合计3888A；都是回收834B，额外2001A，其中1667A重扫描。本轮要消除**可证明不含原始64 ASCII hash** 的局部写入造成的过度失效，绝不能扩大可信GC扫描语义。

## 安全推导与变更范围

原`certify_version_references`扫描每个非活动root对象的**key bytes**和**body bytes**，独立以`windows(64)`匹配当前`installed`集合中完整64小写hex hash（不跨key与body拼接）。因此当前单独操作若只产生 `key.len() < 64` 且 `bytes.len() < 64`，两处均**数学上不可能包含任何64字节原始hash引用**。在该有限语义内，成功小写入不会新增任何可被认证扫描发现的引用，保留此前`referenced`超集一定安全；若旧较长对象中原有引用被新短对象覆盖，不删旧`referenced`只会**过保守拒收**，绝不能因为删除引用而取消旧证书标记。**此证明不涉及任意编码hash、跨字段拼接或未来新增消费者。**

实施时只修改`crates/world-kernel/src/lib.rs`现有`Action::Write`的认证索引失效和收费；`Action`与`WorldCall`类型不变，`program_host`的扫描/GC代码不动，旧索引表示`VersionRefIndex`不变。允许认证保留须同时满足：有当前actor、当前active_hash一致的内核可信证书；`key.starts_with("/self/")`；key与new bytes单独长度严格都小于64；**排除** `/self/active`、`/self/versions/`前缀、`/self/tools/`前缀（root变更和已安装集合变化必须仍全量失效）；调用者依旧必须先通过现有权限/CAS/存储quota及原有`preflight_write`。如果不满足，就走原先在请求动作前保守废证书路径。无法识别旧对象中是否有引用时不得尝试减掉它。所有其他`PublishTool`、Send/Receive/Exec/RestartRoot及环境公共对象更新/交付失效路径保持原状；`ReclaimIndexedVersion`仍需活root/已引用检查和真实付费。

为了使安全快速路径显性收费：**当存在保留资格且CAS写成功时，在原有Write基础费用 `2+ceil(new_len/16)` 上额外收1 A**（严格定长/键长度守卫的保守A服务成本），并在单次Action审计的`gas_charged`报告总费用；如低gas，不得改变对象、版本、储存或证书。如果CAS过期、未授权、InvalidCode或quota拒绝，不收写费，也不新增引用，保留短写证书是安全的（但未证明一般失败动作都应保持索引；非短路径仍遵照原废证书规则）。证书自身的初次全扫描仍真付费`1667 A`；不得把索引构建成本视为免费。

## 执行前冻结的4臂

共享真正S29 P2 world129/protocol42，A12000/epoch、原正确B20任务、研究者真安装三旧候选834B、真实首次CAS激活**相同活动维护根** `88af07516c0e101b260b7950f0e1de5afbfeb5e556952b4bd26b26a2be306815`，限`RepeatBounded(3)`，初始压力 used+2048；每臂`advance_to_true_index`等待**实际有偿**成功Index，然后对同一世界进行普通收费动作，不直接改`WorldState.version_ref_index`。

1. **SHORT_WRITE**：索引后新`/self/noise/s29-churn`写入1B `x`；必须真收费**4 A**（原3+1）、`version_ref_index`仍存在、活动VM无需重建索引，三个GC真Ok合计24A，834B回收，风险对象不存在、根不重新激活，B20、快照一致。总 VM+Action A 应为上一轮QUIET 1884A +4A = **1888A**（此前AFTER_INDEX 3888A）；若环境意外公共对象更新或A不足需报告而非取消安全断言。
2. **LIVE_HASH_PIN**：索引后写入包含`targets[0]`原始完整64 ASCII字符的自有pin，必须仍使旧认证索引真正失效，活动VM首次GC真实Conflict、记录风险b1，三轮不误删仍被pin的版本，B20。不能为优化无害路径削弱这个对照；允许风险状态沿先前冻结逻辑到b1/v2。
3. **FAILED_CAS**：索引后对短无害key正常新建1B对象、成功加收1A；再次用错误`expected_version=None`尝试覆盖当前v0，必须真实Conflict/0 A、索引保留、原值和版本不变；活动VM仍Index一次回收834B。不得将失败写虚构成世界变异。
4. **OLD_REF_REMOVED**：索引之前合法写入64B原始hash pin，使认证扫描真实记录`referenced.contains(target)`；之后用真实CAS v0改成1B `x`，虽然短写允许保留证书，却必须**保持先前的referenced阻止快速GC**（过保守，而不是误删），随后只有通过**实际有偿完整重建Index**才能从当前无引用状态安全回收目标。B20、快照一致。此arm使用原生GC动作，目标root固定且不重复P2任务。

## 验收账本与不允许的捷径

- 完整记录各臂实际Index次数与收费、GC Code/收费、保留/失效状态、Write费用、风险记忆版本、VM A和Action A增量、释放字节、最终storage、B、step和state_hash/snapshot；不允许手动篡改内核索引或把仅研究者调用的GC写成个体自主行为。
- Rust独立定向4 jobs与审计同验收；禁止为了通过测试更改world129、protocol42、A12k、三轮、固定hash、查询/程序预算。红例必须记录，不允许掩饰或只报告最终成功。
- 不因避免1667A扫描而放松外来活引用/active/initial/continuation保护；根程序原始字符串豁免边界沿用旧逻辑。当前优化只解决**短字段的保守假阳性失效**，**不是一般增量GC**、高写频环境期望收益、长期个体策略学习、演化目标自创或正式E001校准。

**科学判据：G0 GO / G1、G2 NO-GO；正式E001未预注册/校准。** 完成真实四臂定向CI后恢复该工作流`workflow_dispatch` only。
