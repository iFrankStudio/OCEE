# DEV — 活动根可信校验器哈希锚与注册表替换（执行前冻结）

状态：**运行前冻结的协议已完成原生验收，下文追加真实四臂结果；冻结判据不变**。基线 main `4c79f08d12b41f5ea0376ea2c8bd0a93735895a1`、[完整结果付费校验报告](./DEV-P2-PAID-EXACT-CHECKER-PROTOCOL.md)及其 [首次原生CI #45103750605](https://github.com/iFrankStudio/OCEE/actions/runs/45103750605)。该Run的VALID仍执行中；上一Run不是4/4，此新实验不替代上一Run结果或自动预言其成功。

## 冻结假设

完整任务重算需要信任校验器本身。此前活动根从研究者可写 `/self/work/exact-fold-checker` 注册表读取64字节程序哈希，不校验其语义或身份；研究者能够有偿安装另一个“总返回1”的纯VM校验器，变更注册表后骗根真实落盘错误结果。本实验不增加更多固定数据点或特殊 VM opcode，而是让**固定可信校验器的内容hash作为活动根不可由该注册表改变的常量**。活动根先有偿Read注册表，提取真实反馈payload，自己通过普通 `EqBytes` 与固定expected hash比较，只有一致才继续读取任务和向校验器正常有偿Exec。非Ok/NotFound的Read不可能拥有与可信hash相等的有效payload。对不一致的hash安全拒绝、不尝试伪checker、不保存工件。

参考校验器仍是研究者编写、符合公开FOLD32定义的完整重算程序，并已通过纯VM的实际有偿安装。哈希锚的来源是**研究者对根程序的事先授权**；不等于无需信任的通用证明，也不是个体自主发现算法、自己构造证书或对根自身遭到篡改的防护。

GitHub实际 world kernel `/self/versions/<hash>` 的 Write preflight 同时要求 canonical Program 字节、`Program.content_hash()` 与hash严格相等，老版本对象不可覆盖；`Exec`仍会再次按hash解码和校验。所以如果研究者攻击者尝试直接把假checker程序写在可信hash地址上，应收到真实 `Code::InvalidCode`。这一存储完整性必须用原生对照验证，不能仅以代码阅读推断。

## 前置冻结四臂

相同代码内容hash的活动根，原世界seed55、开发任务seed2027、delay8、每epoch真实A gas 12000、最多14k tick、S45原生P1学习和第四操作数生成、不更改VM MAX_BLOCKS128。均在研究者安装并注册的工具能力之外无需其它宿主真值输入。

| arm | 原候选 | 校验器注册表内容 | 校验器版本存储 | 根的必要行为 |
|---|---|---|---|---|
| VALID_PIN | 正确FOLD32 | 真实可信参考hash | 真实可信参考Program有偿安装 | 付费Exec checker U32(1)，正确落盘U32(1) |
| WRONG_PIN | 真实FOLD32+sum(byte[2..15])，非真值 | 真实可信参考hash | 真实可信参考Program安装 | 付费Exec checker U32(0)，无成功工件U32(0) |
| REGISTRY_SWAP | 上述真实错误候选 | 恶意“永远通过”校验器hash | 恶意校验器有偿安装，另同时安装可信checker | **根在真正Exec之前发现hash不相等，不执行任何校验器，不落盘U32(0)** |
| MISSING_PIN | 上述错误候选 | 真实可信hash | 不安装可信checker | hash一致，但对预期版本的真正付费Exec反馈 `Code::NotFound`，无成功工件U32(0) |

在REGISTRY_SWAP臂还要额外进行一次对抗性 `Action::Write`：向 `/self/versions/<trusted_hash>` 写入恶意程序规范字节，必须 `Code::InvalidCode` 且可信版本内容不被覆盖；随后世界应保持正常执行，不得将这一研究者故意非法安装计入活动根的Invalid/Denied（应单独记账）。

## 验收

独立核对全部候选真实Exec输入是根生成的16B；可信checker真Exec输入为16B+4B声称值、真实收费及类型化输出；REGISTRY_SWAP与MISSING_PIN分别要求 `0` 和 `1` 次 checker Exec，且恶意hash应 `0` 次Exec；要从世界审计区分任务语义拒绝、身份拒绝和真实NotFound，不能把Gas/VM陷阱当成成功。每臂记录根hash、真实Genesis安装A费、A_VM、A_action、A_total、B/C、Gas拒绝、world step、完整snapshot恢复和无非法根重启。若发生初始化资源不足，不抬预算、不更换冻结seed；记录失败原因并谨慎修复。安装不同不构成跨臂等起点净效用。

## 科学边界

这是**可信引导与内容寻址完整性**机制的负对照，而非使任何工具语义自动可信的程序。根内容若也可被不受约束的攻击者修改，或者最初信任锚本来就绑定恶意程序，则该机制无效。哈希锚应如何由个体自主选择、如何更新、验证旧锚信任、何时值得花费额外A进行完整重算尚未解决。正式E001未预注册/校准；G0 GO、G1/G2 NO-GO。本study定向CI一次性触发、完成后仅手动。

## 2026-11-23 — 原生四臂最终4/4，内容哈希锚与注册表攻击实际账本

[GitHub Actions #45105207973](https://github.com/iFrankStudio/OCEE/actions/runs/45105207973) @ `9d65a04134cb690c371d17ed9c1fae52bb24f051`：**completed/success，四个独立Rust jobs全部1 passed、0 failed**。四组采用完全相同活动VM根的真实内容哈希 `adf01a69992dcb09628d50ff8c7999adbe345f85ae667b17e38a7d4d1b55c368`；根由研究者根据公开参考checker计算初始 pin `333b612826ee95417a647184dae6d3bc243b5da34d2ed1758f05361c05ef7590`，而恶意 checker 是不同哈希 `72d04114a84a65b53e0e6e9335b101b8bb2ae71bcab360ec4bade63f55ec2e32`。相同world seed55 / P2 development seed2027 / delay8 / A12000每epoch / 最多14k自主ticks / S45真实P1学习 / 四U32真实任务，不改VM128块或额度；相同 root 只是安装的真实候选、校验器和注册表不同，不能称“等起点成本对照”。

**可信pin的实际计算链**：在本次根的 `ck_key_reply` 块里，由真实 `WorldCall::Read` 的JSON反馈提取 `payload`，执行标准 VM `EqBytes` 与研究者预置64字节可信hash比较；不相等立即跳 `failed`。在该 World 内核中，非Ok的Read反馈不可能得到等于可信hash的有效payload，因此与hash相等也蕴含真实可见的Read Ok。本次没有新VM指令、没有宿主跳过计费、没有把真值传给根。若相等才调用正常 `Read` 原16B，将原任务候选真实Exec返回的U32编码为4B，付费向指定内容哈希的纯VM校验器 `Exec(16B || 4B)`，仅真实 `Code::Ok` 和 `U32(1)` 后写入真实成功工件；非法/不符/NotFound一律不写。活动根本身虽固定hash，但不是不可变超越世界的可信真值来源。

| 原生观察 | VALID_PIN | WRONG_PIN | REGISTRY_SWAP | MISSING_PIN |
|---|---:|---:|---:|---:|
| 候选主工具真实Exec | Ok=1415179467 | Ok=1415180657 | Ok=1415180657 | Ok=1415180657 |
| Registry Read | 真Ok/可信hash | 真Ok/可信hash | **真Ok/恶意hash** | 真Ok/可信hash |
| 可信 checker Exec | **Ok,U32(1)** | **Ok,U32(0)** | **无，身份闸门先拒绝** | **NotFound** |
| 恶意checker Exec | 0次 | 0次 | **0次** | 0次 |
| 有偿可信checker Exec A gas | 334 | 334 | 不执行 | NotFound动作收费0 |
| 研究者伪造版本写尝试 | 无 | 无 | **Code::InvalidCode** | 无 |
| 根实际返回 | U32(1) | U32(0) | U32(0) | U32(0) |
| 成功工件 | **正确1415179467** | 无 | 无 | 无 |
| 真实初始化A gas | 10277 | 10381 | 10399 | 9662 |
| VM实际A gas | 2533 | 2488 | 2374 | 2468 |
| 动作实际A gas（包含安装） | 11018 | 11169 | 10851 | 10116 |
| **A总gas** | **13551** | **13657** | **13225** | **12584** |
| Gas拒绝次数 | 996 | 1000 | 998 | 983 |
| 世界结束step | 1047 | 1050 | 1048 | 1032 |
| B gas / C成功Send | 0/0 | 0/0 | 0/0 | 0/0 |
| 自主根 Invalid/Denied | 0 | 0 | 0 | 0 |
| 无非法重启、快照完整恢复 | 是 | 是 | 是 | 是 |

REGISTRY_SWAP先**真实**尝试 `Action::Write("/self/versions/<trusted_hash>", malicious_program.canonical_bytes())`，世界内核在可信版本尚未安装前因哈希与规范字节不匹配返回 `InvalidCode`，且不会创建欺诈版本对象；随后合法安装真实可信版本与独立恶意版本、支付所有费用、注册表指向恶意hash。注意这个 `InvalidCode` 是研究者故意试探的违法安装反馈，**不是活动根的VM无效动作**，上表的自主根 invalid=0。实际运行中根从注册表读到恶意哈希时，`EqBytes` 不匹配，**没有调用任一校验器**，不将主工具错误 `1415180657` 落盘。这才是身份拒绝的直接证据，不可混称校验器重算发现错误。

MISSING_PIN真实注册表指向正确hash却未有偿安装对应程序；活动根已通过hash比较且真实提交校验器Exec，在内容寻址版本库里查询得到真实 `NotFound`，安全拒绝且无错误写盘。与REGISTRY_SWAP两种早退不同。WRONG_PIN同一可信参考checker实际返回 U32(0)，原错误偏差1190被完整计算识别；VALID_PIN真实校验器返回U32(1)、结果正确保存。四臂候选工具执行都按 `WorldCall::Exec` 审计，完整状态快照一致。

**限制**：这证实受限确定性VM内的**静态可信锚+注册表完整性闸门+内核内容寻址校验**的研究者编排能力。它不能证明由初始研究者提供的参考checker一定符合公开规格（虽然我们单独测试了若干边界输入），不能阻止攻击者篡改运行的root或最初锚本身，不保证未来自主升级/授权锚轮换；也不意味着廉价证书、没有信任根的数学正确性、个体自主发现验证器或开放式演化。该研究仍是已见seed2027开发集，E001未校准或正式预注册。G0 GO、G1/G2 NO-GO。定向CI在归档提交里恢复仅workflow_dispatch手动。

**前项未结原生job说明**：此前[完整结果校验首次Run #45103750605](https://github.com/iFrankStudio/OCEE/actions/runs/45103750605) 使用不同的未锁定根，前三臂已真实完成并归档，原 `VALID` job截至本次验收时仍未完成。不能用此次新的VALID_PIN成功将旧Run追认成4/4；旧Run应保持其独立真实最终状态。
