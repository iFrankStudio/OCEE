# DEV — S29 已认证引用索引对严格短无害写入的付费保留（研究前冻结）

**状态：先预冻结，原生四臂4/4首次全通过，已归档。** 基线 `5c6759167e0d3f7d2f9b654320c5709c282830c1`。上一轮[同写入不同时间的真实4/4报告](./DEV-S29-CROSS-EPOCH-CERTIFICATE-CHURN-PROTOCOL.md) @e12f53c9：BEFORE_INDEX外部1B Write3A，维护VM+世界Action合计1887A；AFTER_INDEX同内容同目标但在已收费Index之后写入，保守废掉索引，合计3888A；都是回收834B，额外2001A，其中1667A重扫描。本轮要消除**可证明不含原始64 ASCII hash** 的局部写入造成的过度失效，绝不能扩大可信GC扫描语义。

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


## 2026-11-23 — 原生首轮四臂 4/4 完整证据

**[GitHub Actions #45156166086](https://github.com/iFrankStudio/OCEE/actions/runs/45156166086)** @ `da177c8ea2578865057b7a8ecaf64d887f7b8a1d`，4个独立`ubuntu-latest` Rust jobs全部`completed/success`，各 **1 passed / 0 failed**（本轮无编译红例/实验红例）。改动仅在 `crates/world-kernel/src/lib.rs` 的 `Action::Write`预失效条件与成功收费处：当前认证索引绑定actor、active hash，键与新bytes各严格小于64，且不在`active`/`versions`/`tools`关键路径，才能保留可信referenced超集并比普通写额外收1 A。老`certify_version_references`完整扫描、`ReclaimIndexedVersion`活引用保护及世界事件/Send/Receive/Exec等其他失效规则均未放松，未增加Action/VM指令。根内容hash与上一轮完全相同：`88af07516c0e101b260b7950f0e1de5afbfeb5e556952b4bd26b26a2be306815`，world129/protocol42、初始P2任务B20、三候选834B、A12000/epoch未改。

| 本轮arm | 真实索引次数 | 真实GC结果 | 写操作真实收费 | VM A增量 | Action A增量 | 相对维护起点的存储变化 | 终点step |
|---|---:|---|---|---:|---:|---:|---:|
| SHORT_WRITE | 1 | `Ok,Ok,Ok` | `4A`（1B小写入） | **191** | **1697** | **-833B**（834B删除 + 1B新噪声） | 2069 |
| LIVE_HASH_PIN | 2 | `Conflict,Conflict` | `6A`（64B真hash） | 558 | 3367 | +65B（64B真实pin+1B风险） | 2080 |
| FAILED_CAS | 1 | `Ok,Ok,Ok` | `4A,0A`（合法新建+过期CAS） | **191** | **1697** | -833B | 2070 |
| OLD_REF_REMOVED | 2 | `Conflict,Ok` | `4A`（有引用的旧64B改成1B） | 115 | 3355 | -278B（原64B pin→1B、再回收279B） | 2069 |

所有四臂初始根CAS仅一次，`/self/active`根不被重新切换，原P2工具注册和正确结果未改变，B20未重复收费，原生快照`World::from_snapshot`与终态hash一致。OLD_REF_REMOVED后两次GC是**研究者真实直接调用世界Action**，不能称VM选择了重建索引；另外三臂真实GC由同一已激活维护VM发起。

### 关键配对解释

- 上轮已验收的无害**AFTER_INDEX**：index收费1667+1667A，VM A509/Action A3379，合计 **3888 A**；本轮新SHORT_WRITE同在第一次真正Index之后写**相同**1B对象，完成同834B删除，但认证index保持有效，index只收费1667A，VM A191/Action A1697，合计 **1888 A**。新路径相对旧路径**节省2000 A（约51.44%）**，与上轮索引前同内容写入1887 A只差本轮显式定长安全审查费 **1 A**。这是两次不同内核版本下**同一固定世界轨迹的开发对照**，不是对多seed、真实写频或长期性能的统计确认。
- LIVE_HASH_PIN保留老保守保护：完整64B ASCII hash仍清空证书，真实内核GC两次Conflict，VM风险记忆终态b1/v2，旧三版完全未删。SAFE短写规则没有将实际活引用按无害写入处理。
- FAILED_CAS正常新建收费4A；再次用`expected_version=None`对已有v0尝试写入，原生内核真Conflict且0A、认证索引不变、旧对象版本及内容不变，活动VM继续仅一索引完成回收。失败操作不声称产生修改。
- OLD_REF_REMOVED在最初付费扫描前真实存在64B活pin，认证index`referenced`确实包括目标。之后合法CAS将pin改为1B并保留索引，它**继续安全拒绝快速删除**（保守过时的正引用），未发生误删；研究者付费重新全量索引后，新证书才确认未被引用，真GC8A删除那个279B旧版。此安全反例同时证明该优化只是**增量保留证书**，还不具备精确逐对象引用计数或更新后解除旧引用的能力。

### 边界与科学闸门

本研究只解决原始哈希匹配模型下严格短写入带来的**无根据证书失效**。其它≥64B内容、跨对象拼接、编码指针/外部持有、长程异步引用及任意环境服务均未证明；旧“引用由任意长对象被覆盖”场景会保守保留旧标记直至真实新扫描。预设三版本候选、真实GC时机、25%压力门槛和三轮维护根仍是研究者供给，个体并没有自主发明资源策略或内生发现回收目标。**G0 GO / G1、G2 NO-GO，正式E001仍未预注册/校准**。这次只验证专项4/4，不能追认整个Rust workspace。专项工作流在成功归档后改回`workflow_dispatch` only。

下一个高信息量研究可以是**有版本的逐对象引用差分**，但必须先证明移除旧引用不会漏掉其他对象、键名及在途消息；变更须与本轮保守索引作为固定对照，量化额外索引元数据空间和每次写入A开销，尤其保留长内容及多引用的负对照。
