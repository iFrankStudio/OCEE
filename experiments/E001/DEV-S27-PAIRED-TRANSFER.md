# S27 — 同一初始包工具继承的原生配对试验（开发集）

**2026-11-19 · exploratory / developmental；不是正式 E001，G0 GO、G1/G2 NO-GO。**

## 方法和目标

[S27 Rust 测试](../../crates/organism-seeds/tests/s27_transfer_pairs.rs) / [独立 CI](../../.github/workflows/s27-transfer-pairs.yml) 沿用 S26C 完全相同的 root+S19(P1)+S23(P2) 专家程序包，所有组预安装与资源计费相同。活动 root 仅凭公开 HELLO 选择专家，专家在真实 C→B 任务中自行生成、封存编码工具。

Source 世界：P1 或 P2，world/protocol seed42；Target：P2 world128/protocol42。同一个 Target 配置和相同初始包构建三种轨迹：

1. **C 冷启动**：没有跨 episode 学习状态。
2. **D 跨家族**：仅迁入 P1 个体创建的可执行工具及其原生名字空间 p1-eval-frame（真正的受计费 WRITE，不伪装成 P2 工具）。
3. **D 同家族正对照**：仅迁入 P2 个体创建的 p2-eval-frame 程序及 registry 引用。

不是将旧世界的 secret、B 结果、邮箱或 VM continuation 整体迁入。所有路径真实世界 tick，B 只计费20且 FOLD32 与独立算式一致。研究端记录 A gas、通信 byte-hops、world tick、导入成本、事件日志与快照哈希一致性。

## 可证伪的因果断言

- 同家族可兼容工具必须通过 Target 真实 EXEC 使用，不重复执行冷学习；C 中应存在冷候选生成和测试，导入成功仍需计入成本。C 可能自行生成**相同哈希**的程序，因此单看哈希相同不能证明存在继承；须看导入 WRITE 和候选/执行审计。
- 跨家族 P1 工具按 P1 原生名字空间保留，Target P2 不应把它暗中当成 P2 编码器使用。若未执行，则 D 的 B 任务成功**不能归因于该工具**。这种负对照可识别当前机制中跨家族结构重用的空缺，而不宣称生命演化失败。
- 同源程序包下、相同目标 seed 与协议实例的 C/D 是有效配对；不同 Source/Target 家族的跨组成本不能直接混比。
- 只有跨家族 P1→P2 实测 C/D 行导入 [s26_paired_audit.py](./s26_paired_audit.py)；同家族 P2→P2 正对照另外报告，不混进跨家族脚本。CI 归档原始 CSV，包括失败记录；没有原生结果前不填写观察数值。

## 解读与下一步

预计同家族继承体现净收益，而 P1→P2 原生工具不会被现有 P2 专家消费，甚至增加导入成本。如果实测与预期不符，按原生日志定位和修复，不能选择性报告成功。

**即使 S27 全通过，也只是在研究者预置的语法假说/专家程序包下的状态继承。** 下一步是让活动个体产生真正协议无关、两家族都可以功能性调用的工具，并以移除工具的反事实证明其迁移价值；再考虑 E001 的 G1 校准。研究者先验与自动生成能力必须持续分开。


## 2026-11-19 原生核验结果（最终）

- 首轮 [#44558035166](https://github.com/iFrankStudio/OCEE/actions/runs/44558035166)：Rust 1/1 通过，但 CSV 在 Cargo test 的 crate 工作目录下保存、审计脚本从仓库根读取，workflow 因 FileNotFoundError 失败；不得视为完整 CI 成功。修复提交 [022df157](https://github.com/iFrankStudio/OCEE/commit/022df15700171d473c003ef0e6d3554fc63f0ae3) 将 CSV 写入 github.workspace/target 的绝对路径。
- 最终 [S27 定向 CI #44559169797](https://github.com/iFrankStudio/OCEE/actions/runs/44559169797)：**completed/success**。Rust 一项关键跨世界因果测试通过、Python 配对分析成功、真实原始 CSV 工件上传成功，下载地址：[s27-cross-family-paired-dev](https://github.com/iFrankStudio/OCEE/actions/runs/44559169797/artifacts/14548366400)，artifact SHA256 为 `cb7a32babcdd83e58b556e402b277795b4c9327696b99d8f4d4111b73c568369`（GitHub artifact 压缩包 digest，不是 CSV 文件的摘要）。
- 使用同一个 root `03a2ca2e477b6d9985fd16e0ed457e67ceeafbb5c1d9789623821a343d7e3a46`，source world/protocol seed42；target world128/protocol42：**冷 C** A21,833 / tick1163；**跨家族 P1→P2 D** A21,895 / tick1163，迁入62 gas，来自 P1 的可执行工具在 P2 中实际 EXEC 次数为0；**同家族 P2→P2 D** A16,671 / tick1087，迁入102 gas，来自 P2 的工具确实 EXEC，较冷启动节省5,162 A gas（23.64%）。所有组 B gas20，真实 FOLD32 与独立参考一致。
- 跨家族 CSV 通过 s26_paired_audit.py：paired_seeds=1，C/D 成功各1，配对成功差0，D 成功且声称工具因果使用数0，两组都成功时 A 成本 D−C=+62，B gas、byte-hops、ticks 均差0。只报告单个开发 seed 的事实，**绝不声称统计显著性或 H3 确认性通过**。
- **机制结论**：现有模型能以可测净收益继承**同一家族相同协议实例**的封存程序，但跨 P1→P2 原生命名的程序未被当前 P2 专用执行策略消费；单纯跨家族搬运已产生负资源收益。G1/G2 仍 NO-GO。下一切片需真正跨家族复用协议无关、经功能自测的子程序，并有工具移除反事实。
