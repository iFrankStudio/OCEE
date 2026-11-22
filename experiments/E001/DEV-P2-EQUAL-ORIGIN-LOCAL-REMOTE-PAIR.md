# A-only / A+B 同起点完整任务开发集配对实验

**状态：执行前固定代码、两路和判据；首次原生CI尚无结果。**

先前的[同操作数边际成本](./DEV-P2-MATCHED-LOCAL-FOLD-COST.md)只在远程成功后额外运行本地程序，缺少整个学习任务的同起点控制。此次实验共用世界seed55、已见开发P2协议seed2027、全程delay8、初始真实A12000 gas/epoch。研究者一次性装载**同一份**P1工具、S45 P2专家、完整根程序以及通用纯VM本地四U32 FOLD32工具，所有Write真实收费，不补满A余额。

根程序对两个臂的hash完全相同，执行原有公开任务学习/验证/生成第四操作数，直到正常 `switch_active` 才读取 `/self/compute-route`。同一付费genesis世界的精确snapshot分叉，外部研究者以**同样费用**的一字节Write设置L或R。L分支是原root主动Read它生成的16字节、Exec已付费本地工具、Write相同格式结果并返回1；R分支进入原有S40版本CAS切换、原S45协议真实B远程计算与ACK/RESULT来源确认。**路由选择来自研究者、不是个体自主学习**；A-only仍具有同等已安装的P2专家与世界中存在的C/B，只是实际运行严格不得给C发送消息或扣B；R臂同样付费装载但不使用本地工具。

两臂各最多14000个自主tick，均必须真实产生四个U32、将结果写入同一个self对象并返回`U32(1)`，独立fold32正确、快照哈希一致、无VM错误与越权重启。L必须本地VM Exec恰一次、C发送0/B0；R必须B24一次/真实RemoteEval唯一/对应Request ID的ACK和RESULT确认、本地Exec0。两臂打印相同机器行`P2_EQUAL_ORIGIN`，含分叉前hash、两个程序hash、创始收费、A VM与动作gas总额、B/C消息、完成步数及Gas拒绝。红色CI必须保留，不得按结果选种子或放宽测试阈值。

**解释上限**：该研究是相同的初始策略能力和同起点的**研究者指定L/R路由**对照，更接近完整任务同起点成本比较，但不是个体自己发现节约资源或正式E001 A/B/C/D适应能力对照。不同末端路径行为正是因果干预；同等付费安装的工具均由研究者预置，不能计为新工具自我创造。结果只对已见P2实例/四U32工作项有效，不能推广到复杂B任务或跨家族迁移。科学仍G0 GO，G1/G2 NO-GO。

## 2026-11-22 — 首轮负例保留与后续有界资源修复（原始CI仍在完成R臂）

首次[Run #44994405005](https://github.com/iFrankStudio/OCEE/actions/runs/44994405005) @ `bb401d1a989802fb9579b01f9ddb292882909aa2`：本地L臂在第1025步`finished=true, return=U32(0)`，B0、C-SEND0、没有任何本地Exec成功，原始安全与初始化约束满足。分叉前世界hash为`fbc7c3ea81d07491f9543465ace977a2fc5ef2a215b845e482bc820fa6a1b286`，研究者相同工具初始化gas9435、路由Write3、A总gas12015；L臂第1025步主动失败，**不应标记为已完成全任务配对**。R臂于此记录创建时尚在运行，结果需另记。请保留此开发红例和后续修复实验的源码区别。

分析高优先级机制缺口：根的本地`WorldCall::Exec`是**不可跨tick继续的隔离原子调用**，其执行预算为`min(A余额,VM_SLICE_GAS)`；当A余额不足时返回`Code::Gas`，原始实验的本地分支将任何非Ok立即判为失败，可能在初始epoch耗尽时误放弃任务。它不同于可跨tick持久化的活动根continuation。修复版不改变L/R选择、初始程序、样本seed、成本许可或成功判据；只对本地Exec真实`Gas`反馈添加总数2048的`RepeatBounded`（重读真实16字节再尝试Exec），其它状态仍fail-closed；不会调用B、补A或延长14k活动tick。额外输出`local_exec_gas`实际尝试历史以确认故障机制。此属**看到开发失败后的修复**，不能回写为首次预注册成功；需原生重新验证两臂。

## 2026-11-22 — 首轮R正例与L阴性结果封存

[Run #44994405005](https://github.com/iFrankStudio/OCEE/actions/runs/44994405005) @ `bb401d1a989802fb9579b01f9ddb292882909aa2` workflow completed/failure，R、L两独立原生jobs：

| 路由 | 任务 | 初始genesis/route A | A总gas | B gas | C成功SEND / 带宽 | Gas拒绝 | 终止step |
|---|---|---|---:|---:|---|---:|---:|
| L | **U32(0)失败**；无成功本地Exec | 9435 / 3 | 12015 | 0 | 0 / 0 | 981 | 1025 |
| R | **U32(1)成功**；结果1415179467 | 9435 / 3 | 47350（VM36572、动作10778） | 24一次 | 61 / 1726 | 2277 | 3383 |

两个臂完全相同的初始分叉前world hash `fbc7c3ea81d07491f9543465ace977a2fc5ef2a215b845e482bc820fa6a1b286`、共同root hash `46c40f910b11f26316542c689169526376e7bb9b144b919644009fae4794c28a`、P2专家hash `0f9163bdff40a2ac5cbee63586353ea394f639d6dc7cc190ac34c811634c518e`。R的真正RemoteEval u64 ID118与RESULT来自内核确认，L未发送C/B。**不能以L失败时的12015 A gas与R成功的47350比较净效用。**

后置有界Gas-only重试修复另在 `2bd9fc9255574093d5c2df55304b04e585911a20`，不改变原样本与资源，追加`local_exec_gas`日志核实低余额下隔离Exec失败的机制；R也必须在修复后新root hash下重新验收，不能以首次R成功代替。
