# S28：跨家族可执行通用子程序的功能性继承（开发实验）

**状态：实验代码和定向 CI 已提交，原生结果尚待核验。本实验不是正式 E001，也不构成 G1/G2。**

## 从 S27 的负面证据出发

[S27B 修订定向 CI #44659637319](https://github.com/iFrankStudio/OCEE/actions/runs/44659637319) 已成功。其原生日志显示 S27 sham_control：冷启动 A21,833；工具等成本导入但不可见 A21,839（导入102，扣除导入后的运行部分比冷启动 **少96**），两者 tick 均1163；兼容可见 P2 工具 A16,671。说明**导入动作能改变微观调度和计量，不能把 cold−D 直接当成严格工具因果效应**。同源 sham 对照比 cold 更有说服力。

S28 不再以协议编码器跨家族复用为目标，而尝试最小、可证伪的**协议无关可执行子程序复用**。

## 新的机制与先验来源

- [S28 根程序](../../crates/organism-seeds/src/s28_cross_utility.rs) 沿用 S26C 同一公共 HELLO 路由和两份**在所有组等额安装**的 P1/P2 既有专家。根程序新增只依赖公开 `/public/E001/job0` 的字节长度检查入口；不直接读取私有 protocol、B 账本或目标答案。
- **研究者预设**：一个单节点、U32 输出的 `ConstU32(0)` 代码模板，以及把该节点替换成 `Len(0)` 的候选改写位置、目标长度12、检查策略与注册名字。并不是个体自行发现 Len 运算、未知语法或新目标。
- **活动 VM 真正执行的行为**：从公共工作对象读取 `values_le`，若没有注册工具则在 VM 中执行 `RewriteOp`，计算内容哈希，以受计费的 WRITE 持久化不可变版本和 registry。随后通过受计费的 `WorldCall::Exec` 调用可执行程序，检查返回的 U32=12，成功才进入 S26C 的公开 HELLO 路由、受计费切换至 P1/P2 专家并完成 C→B FOLD32。
- 与 P1/P2 报文结构无关的**工具契约**是：Bytes 输入 → 字节长度 U32。工具内容哈希是版本身份，registry 的正确位置是调用约定；两种环境都要真正使用它，而不只是携带文件。

## 配对与消融（单个开发世界实例）

1. **Source P1**，seed42：在真实活动 VM 中生成通用工具，执行并成功完成 B，记录生成版本写入、注册与 EXEC 的审计。
2. **Target P2 C cold**，world seed128/protocol seed42：无继承工具，当前根程序需自行生成同类代码并执行，然后完成 B。
3. **Target P2 D transferred**：沿用**完全相同的初始 root + 专家程序包**，并将 Source P1 活动 VM 真实生成的工具及 registry 通过两个受计费 WRITE 导入 P2；目标 VM **只能读取、EXEC，不能重新生成该版本**，并完成 B。
4. **Target P2 sham**：以**完全相同的程序字节/导入成本/写入步数**导入，但注册于仅一字符不同的不可见 key；应主动创建新的版本、执行并完成 B。D 与 sham 在启动后比较相同 root、相同写入时间及主要资源账本，而不是直接假定其成本相等。
5. **Target P2 poisoned**：正确 registry 下提供一个类型正确、可 EXEC 但输出常数0的程序。若功能校验有效，应停在根程序且 B gas0（禁止伪造成功）。这是功能门控消融，**成本不与真实 D 等额**，只证明调用结果影响后续行动。

所有完成远程任务的世界都要核验 B gas=20、FOLD32 独立正确、快照重载状态 hash 一致。重点审计 root 切换 **之前**工具真实成功的 EXEC；冷启动也可能生成与继承工具相同的哈希，故需要生成/导入的行动来源与事件序列，不能只凭哈希和 EXEC 次数推断来源。

## 报告界限

如果 P1 生成的通用可执行工具在 P2 被消费，并优于相同导入节奏的 sham，才可报告**受先验限制的跨家族功能性复用**。它仍是**研究者播种模板的 VM 改写与注册**，不是开放式演化、未知协议自动发现、算法自创新，也不能从 1 个种子推广到其它环境或推动 E001 G1。若负对照失败，保留失败日志并修订测试/策略；**不调整已测 seed 使结果好看**。

[原生测试](../../crates/organism-seeds/tests/s28_cross_utility.rs) · [定向工作流](../../.github/workflows/s28-cross-utility.yml) · E001 G0 GO / G1、G2 NO-GO。


## CI 首轮异常与内容寻址冲突（保留失败证据）

- [首轮 S28 CI #44669843136](https://github.com/iFrankStudio/OCEE/actions/runs/44669843136) 的 Rust 编译通过，但测试失败，最初 `U32(0)` 没有足够详细的轨迹。增加诊断后，[第二轮 #44670196068](https://github.com/iFrankStudio/OCEE/actions/runs/44670196068) 原生日志证实真正失败的是 **P2 sham**：step8 已有相同内容哈希的版本，VM 在未发现正确 registry 后自主构造同字节代码，写入 immutable `/self/versions/{hash}` 收到合法 `Code::Conflict`，原实现把此状态当成致命失败（step10、B gas0）。这不是源 P1 或继承 D 的远程计算失败。
- 已通过 [修订 e7f515a](https://github.com/iFrankStudio/OCEE/commit/e7f515a12d338dbb7870ceb55bb4ffdf4f4593d7) 在根程序图中增加 **仅对版本 WRITE 结果 Code::Conflict** 的分支，允许根据自身计算的哈希继续注册并 EXEC 已存在的不可变程序；世界内核对版本内容执行哈希校验，其他结果（如 Quota、Gas、Denied）仍失败。修订来源是研究者显式工程修复，不可算作个体自修复。
- [测试修订 da96e13](https://github.com/iFrankStudio/OCEE/commit/da96e136b5707070c08d6170718786a377345be4) 将 sham 的“成功写入新版本”误断言改为“**确实尝试了自主生成，但遭遇合法哈希冲突**”，以审计结果区分生成尝试、成功创建和跨 episode 继承。相同内容哈希本身没有来源信息。
- **新 S28 定向 CI #44670645988**：[查看](https://github.com/iFrankStudio/OCEE/actions/runs/44670645988)。截至本条撰写仍未完成，故不得从先前失败轨迹推导整个 S28 已通过。


## 原生定向验收：2026-11-20

- 修复后 [S28 专用 Actions #44670645988](https://github.com/iFrankStudio/OCEE/actions/runs/44670645988) **completed / success**，原生 Rust 单项主试验 **1 passed、0 failed**，执行 **218.17 秒**，head [da96e136](https://github.com/iFrankStudio/OCEE/commit/da96e136b5707070c08d6170718786a377345be4)。与此前两个失败运行的证据不可混淆。该 SHA 之后的文档修改不更改测试代码。
- 同一 root `a12ad82a5ab980eb14f003ea2410f7263a314ac920af4ef4e3553580eba6eca0`；通用工具 hash `337be40be6f438c015af1f21750325b76654ccac010628c2a0fe28d08e9d81cc`。Source P1 的 `RewriteOp`、不可变版本 WRITE、registry WRITE、成功 EXEC 及实际 B 结果由世界动作审计验证；Target P2 带来的完全相同程序被 root 在自主切换到 P2 专家**之前成功 EXEC**，且没有自己重建该版本。
- Target P2 world seed128 / protocol42，**完整 A gas 包含初始 root+P1/P2 程序包和实际导入开销**：cold **22,477 gas / 1174 tick**；D inherited **22,349 gas / 1170 tick**，导入 **24**；等成本 sham **22,486 gas / 1174 tick**，导入 **24**；poisoned **5,366 gas**，导入 **24**。**D − sham = −137 A gas**（约 0.61%）；D − cold = −128 A gas（但 cold 的写入时间不同，因果估计优先 D−sham）。
- 冷、D、sham 三组均经活动 VM 真正 C→B 完成 FOLD32 且 B 各20 gas，持久结果与独立参考一致。poison 的程序类型有效且成功 EXEC，但 `ConstU32(0)` 违反 root 的长度12校验，终态失败、活动程序保持 root、**B gas0**；不能把 poison 的低 gas 当作任务完成或继承收益。
- sham 并非没有导入工具：它以相同的程序字节、写入步数和费用安装在不可见的一字符不同 registry 下。随后 VM 独立做出相同 `RewriteOp`，得到相同哈希；版本 WRITE 返回 `Conflict`，root 接受此**指定错误**而后重新注册，因此 sham 实际完成了代码重建决策和使用。该路径严格说明同哈希不能证明个体从哪一 episode 学到程序。
- **证据等级严格限定**：一个目标种子、一个人为设计的长度检查任务、研究者预置的 `Len` 候选、目标条件与生成模板；目前仅说明**可执行、协议无关、类型化子程序可以以跨家族身份被实际继承并带来小规模净计量收益**。这不是未知语法自学、未提供候选算子时的发现、普适迁移或开放式演化；不足以通过 G1/G2。全 workspace CI 对本 SHA 仍需另外核验，不得把定向 CI 叫全量。
