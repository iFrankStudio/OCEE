# S18 — 活动 VM P1 发现、远端执行与 RESULT 识别消融（开发集）

日期：2026-11-18。**仅开发实验；G1/G2 NO-GO；正式 E001 未启动。**

## 已有可验证基线
- [b5517a9](https://github.com/iFrankStudio/OCEE/commit/b5517a9012c8c0273d2cbc6b28616996aa6c3402) 的活动 VM 在已知 P1 字节填充、校验与报文骨架下，从公共 HELLO 推导实例校验键，通过有界诊断扫描寻找 REMOTE_EVAL opcode；随后通过普通 SEND/RECEIVE 提交三 u32 FOLD32 工作项。其 [CI #44310102801](https://github.com/iFrankStudio/OCEE/actions/runs/44310102801) 已完成成功。
- [6f428e6](https://github.com/iFrankStudio/OCEE/commit/6f428e6d5b5754f62667d021a73cd435743ede33) 又在同一 VM 内添加 RESULT opcode 扫描、响应结构检查、request_id 核对及结果持久化；该提交的 CI #44311202557 在本次初查时仍未完成，**不得用旧 CI 证明新阶段**。

## 新增机制消融（本轮）
- [67fcf96](https://github.com/iFrankStudio/OCEE/commit/67fcf966b988bf9f83a6d796b0de5416ddf53c26) 添加仅将 RESULT 响应长度期望从 14 改为 15 的受限程序变体，保持先前 opcode 探索、实际 B 执行和其他状态检查不变。
- [9aeb3c4](https://github.com/iFrankStudio/OCEE/commit/9aeb3c408dd6ad98a17e0c3a2873e5fd273ba4f3) 添加单个聚焦测试：seed 42 下应已保存 REMOTE_EVAL 探索记录及 ACK，B gas 应为 20，但不保存最终结果，VM 返回失败。这隔离“完成远端计算”与“正确识别并持久化结果”，防止只凭 B gas 误判闭环成功。
- [本轮 CI #44311562315](https://github.com/iFrankStudio/OCEE/actions/runs/44311562315) 初次检查为 pending；**尚无通过或失败结论**。后续先读最终状态和日志，失败时定位修复；通过后更新计量数据及检查点。不要为此反复推送仅用于等待 CI 的提交。

## 能力来源与限制
P1 分隔符/转义、内层布局、XOR 规则、112 次候选范围、BadBody 选择、固定 FOLD32 请求形状、ACK/RESULT 字段和响应匹配策略均为研究者初始程序先验。隐藏 opcode 与工作项数值不预置；实际搜索由活动 VM 执行并计费。该测试仅验证预置识别规则的必要性，**不证明陌生协议语法自主发明、工具创新、跨 episode 学习或开放式演化**。保留诊断反馈失效、双来源共同造假、错误继承等既有负面结果。
