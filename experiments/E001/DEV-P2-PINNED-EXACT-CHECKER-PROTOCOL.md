# DEV — 活动根可信校验器哈希锚与注册表替换（执行前冻结）

状态：**运行前冻结，尚无原生结果**。基线 main `4c79f08d12b41f5ea0376ea2c8bd0a93735895a1`、[完整结果付费校验报告](./DEV-P2-PAID-EXACT-CHECKER-PROTOCOL.md)及其 [首次原生CI #45103750605](https://github.com/iFrankStudio/OCEE/actions/runs/45103750605)。该Run的VALID仍执行中；上一Run不是4/4，此新实验不替代上一Run结果或自动预言其成功。

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
