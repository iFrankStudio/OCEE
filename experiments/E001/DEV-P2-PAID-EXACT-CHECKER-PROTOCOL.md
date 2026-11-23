# DEV — 付费完整结果复核与校验器信任边界（冻结）

状态：**运行前冻结的开发实验**，最新基线 `9f8e08628ad0f5367e1474f8f35284be30a743b6`，[双坐标四臂原生实证 #45102384014](https://github.com/iFrankStudio/OCEE/actions/runs/45102384014)。不是正式E001预注册；G0 GO、G1/G2 NO-GO。

## 研究方向

单点零见证与多个固定变形关系，无论怎样增加，都会存在不同任务输入上与全部局部验证共形的共同错误。用更严格的任务局部校验器直接检验**整个原16B输入和声称的4B输出之间的FOLD32关系**，不做进一步的黑箱采样。要求活动根从已有真实本地Exec Ok反馈解出 U32 claim，经通用PackU16Le构造该值4B小端表示；有偿从真实 `/self/work/remote-values-le` 读取个体自己生成的完整16B输入，并与claim拼接成20B，向另一份**研究者预置、已正常有偿安装的受限VM程序**发出真正 `WorldCall::Exec`。其返回真实类型化U32(1)方可写入结果，否则fail-closed；校验器hash须从有偿Write的注册表里经根真正Read取得，不允许测试宿主替根填充反馈或更改目标答案。

严格的校验器是与生产工具在算法结构上不同的一份**完整计算实现**：按任务定义FOLD32，把四个小端U32按位置旋转相加，再比较给定claim，返回1/0。研究者维护和测试这个程序；虽然它通过纯VM执行且付费，不是世界内核的新特权oracle，但这是一份**具有可信规格依赖的参考校验器**，并非一个可以脱离信任前提的通用数学证明，也不是个体自行发现算法。完整重新计算也不保证比原任务计算更便宜，必须报告真实成本。

## 原生预冻结四臂

相同活动根hash、世界seed55、开发P2 seed2027、delay8、A12000/epoch真实收费、14k自主tick上限、原S45 P1学习/合成操作数/结果工件与严格VM MAX_BLOCKS=128均不变。四臂在根之外仅改变研究者正常付费安装/注册的工具或校验器，不能把跨臂A总成本解释为等起点因果差。原候选工具仅执行一次原任务，不要求第二份原工具再投票（验证器本身进行完整独立任务计算）。

| 条件 | 候选工具真Exec | 真实有偿校验器 | 预期根处理 |
|---|---|---|---|
| VALID | 正确FOLD32 U32 | 正确完整重算的checker U32(1) | 正确落盘，U32(1) |
| PLUS_ONE | 正确值+1，Exec Ok | 正确checker U32(0) | 不落盘，U32(0) |
| RESIDUAL_FAULT | FOLD32 + Σbyte[2..15]，Exec Ok且不为正确值 | 正确checker U32(0) | 不落盘，U32(0) |
| COMPROMISED_CHECKER | 同上残余错误工具，Exec Ok | **研究者故意安装“总是返回U32(1)”的损坏checker** | **假成功、错误落盘**，说明必须认证校验器/规格，不是通用正确性证明 |

所有成功路径必须在审计中验证真实的20B输入=世界生成16B||主工具实际U32输出的小端4B；真实 checker program_hash 与研究者正常有偿注册的hash一致，工具所有Exec的结果、世界gas、VM gas、A总费、genesis成本、B gas、C SEND、world step、snapshot恢复和Invalid/Denied。正确结果只在测试端外部计算，不泄漏给活动根。失败路径也必须有一个真实校验器 `Exec Ok U32(0)` 才能证明语义拒绝，不能以预算不足/VM陷阱冒充检测。

不得抬高A gas、放宽时间、变动任务输入/seed或VM验证器。若初次运行红灯须归档失败与修复链，不把后验修复追认成首次通过。CI一次性仅跑四臂独立Rust jobs，验收后恢复workflow_dispatch。原E001整体未校准预注册，不新增G1/G2能力宣称。

## 推进价值与新的边界

可真正独立地核验所有FOLD32输入的声称结果（在校验程序确实符合公开规格的假设下），而不局限于固定样例；研究将揭示这一方法与单次原工具计算的真实A成本，以及可信验证器被篡改时的可证伪失效。**完整重算不等于廉价验证、校验器可信不等于自证正确，固定研究者策略不等于自适应或开放演化。**

## 2026-11-23 — 首次原生中途账本（3臂完成；正常组仍运行）

[原生独立 Rust 四臂 Run #45103750605](https://github.com/iFrankStudio/OCEE/actions/runs/45103750605) @ 1eeb01cccf57927a3b02689ded21b84039dd8d81：截至本段编写时，**RESIDUAL_FAULT / PLUS_ONE / COMPROMISED_CHECKER 三个 jobs 已各自 success/1 passed；VALID job 仍in_progress，故整项尚不能宣称4/4或 Run completed/success**。这不是实验结论的后验调整，保留原协议、seed2027、预算、四臂判据和研究者预装校验器。在正常组完成后须补录准确结果；其间不允许将缺失证据当作通过。

三臂真实同一活动 root SHA256内容哈希：`03a0e04248abef1c4520c5633f4c84daef456e783d5a29e71952c7d7e41fe0d6`。此活动根让研究者安装的候选纯VM针对**自行生成的真16字节**真实Exec，然后以两段PackU16Le组合被声称的4B结果，并付费Read自身工作输入与checker's注册hash，对这20B调用另一个真实付费 `Action::Exec`；其返回Code::Ok、payload U32(1)才写成功工件，U32(0)则不写，不存在世界privileged oracle。

| 已完成独立原生审计 | PLUS_ONE | RESIDUAL_FAULT | COMPROMISED_CHECKER |
|---|---:|---:|---:|
| 候选真实Exec Ok | 1415179468 | 1415180657 | 1415180657 |
| checker真实Exec Ok | **U32(0)** | **U32(0)** | **U32(1)：伪通过** |
| 本次checker真实世界收费A gas | 334 | 334 | 2 |
| 测试端独立正确值，VM不可读取 | 1415179467 | 1415179467 | 1415179467 |
| 活动根返回 | U32(0) | U32(0) | **U32(1)：假成功** |
| 结果落盘 | 无 | 无 | **1415180657（错误）** |
| genesis收费 A | 10273 | 10373 | 9672 |
| VM A gas | 2484 | 2485 | 2529 |
| 世界动作 A gas | 11013 | 11161 | 10131 |
| 总A gas | **13497** | **13646** | **12660** |
| world最终step | 1046 | 1050 | 1033 |
| B gas / C成功SEND | 0/0 | 0/0 | 0/0 |
| Invalid/Denied / 恢复快照 | 0/一致 | 0/一致 | 0/一致 |

`RESIDUAL_FAULT`候选沿用上轮已有的两个坐标仍误判的 `FOLD32 + sum(byte[2..15])`，真偏差1190；在本次**完整输入参考复核**下被成功拒绝（Code::Ok,U32(0)）。`COMPROMISED_CHECKER`仅把正常校验器换成独立有偿安装、注册的`return U32(1)`合法纯VM，根完全相同且收到了真正的付费Code::Ok响应，但错误工件确实被存储：说明**注册表内的哈希/实现语义若不可信，完整复核同样无法建立真正信任**。校验器虽然通过独立VM执行，但它本质上完整重算参考任务，不存在廉价证书或个体自主演化。这些组初始安装字节和genesis差异很大（尤其篡改者），不能横比A总成本推断净收益。

原生工作流现在重置为`workflow_dispatch`手动，历史旧实验不被重新触发；初始Run的最后一个VALID job仍可在GitHub完成且须另行审计。G0 GO、G1/G2 NO-GO；正式E001未校准/预注册。

## 2026-11-23 — 初始完整校验器CI最终状态修订（不重写当时记录）

[初次原生 #45103750605](https://github.com/iFrankStudio/OCEE/actions/runs/45103750605) 后续最终变成 **workflow completed/cancelled**：原 `VALID` job **completed/cancelled**；其余 `PLUS_ONE`、`RESIDUAL_FAULT`、`COMPROMISED_CHECKER` 三独立 jobs 各 `completed/success`，原生证据实际分母仍为**3完成+1取消，不是4/4**。上文“当时仍在运行”是当时的历史状态，不能自动追认；新的pin根的 [#45105207973](https://github.com/iFrankStudio/OCEE/actions/runs/45105207973)是不同根代码与不同实验，自己的4/4不修复这个Run的取消。
