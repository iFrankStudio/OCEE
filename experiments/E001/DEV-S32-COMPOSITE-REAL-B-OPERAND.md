# S32 — 经验选择的组合程序直接成为远程 B 操作数

**2026-11-20，开发性研究；G0 GO，G1/G2 NO-GO。未经正式 E001 确认性预注册。**

## 研究命题与预置条件
S30 个体已能在研究者预置的 18 种“两个一元节点 + 一个二元组合节点”程序图中按两组公开样例选择、经 RewriteOp 生成并跨家族继承；S31 将研究者播种的 Len 程序输出放入真实 B 工作 count。S32 要验证：**同样经经验选择的多节点程序输出，能否进入真实 C→B 的计算操作数，并因干预造成真实 B 结果变化？**
明确先验：18 候选语法、图插槽、追加第四个操作数的任务规则、U32 小端编码、两份 P1/P2 协议专家与本地模拟 C/B 都由研究者预置；不是自由发明语法或真实互联网工具使用。

## 活动 VM 中的完整数据路径
1. P1 VM 真实读取公开 compose 的双标签，生成、保存并真实 EXEC 多节点程序；P2 通过受费 WRITE 导入字节与 registry，并按目标公开样例再次验证，错误旧程序可被 S30 的预置恢复策略搜索和 CAS v0→v1 替换。
2. 目标 VM 对自己的新输入 `/public/E001/transform.held_out_input` 执行所选组合图，获得 U32；原有三个公开 U32 与输出经两次 PackU16Le 拼成 16 字节，受费 WRITE 到 `/self/work/remote-values-le`。真正的 `hidden_compose` 只在实验后用于独立审计。
3. S31 的另一个由活动 VM 改写而成的 Len 工具实际 EXEC 这 16 字节，得到计数4，受费 WRITE 到 `/self/work/remote-count-le`；同一初始 root 根据公共 HELLO 自主选择 P1/P2 专家，两份专家均必须 READ 两个共享对象，真实执行自己的帧程序并 SEND 到 C/B；B 实际执行四操作数 FOLD32，B gas24。

## 可证伪对照与原生实测
- 首轮 [CI #44686476742](https://github.com/iFrankStudio/OCEE/actions/runs/44686476742) Rust 编译完成但静态 VM 类型检查失败：WorldCall::Write 延续的顺序是先 carried U32，再追加 feedback Bytes，新区块错误颠倒。通过 [修订 e619b212](https://github.com/iFrankStudio/OCEE/commit/e619b212ddbd9faefe3c2914431f577c7df7810c) 修复，不更改原 seed/判据。失败是接口错误，不是学习行为失败。
- 修订后的 [**S32 原生 CI #44686613065**](https://github.com/iFrankStudio/OCEE/actions/runs/44686613065) 已 **completed/success，Rust 1 passed / 0 failed / 84.59s**。冻结初始 root hash：`ad0e5ab7840e26c464442fc4283accc098e04f113bd879df20542856b291b39d`。
- P1 world1 → P2 world19：所继承的 VM 生成组合代码 hash `ccbec864ef8a002aa15e07a541cfca57200fa8770f51020274f1b1d359aa43c5`，目标未重新生成该哈希，真实第四操作数 **52**；导入29 A gas；P1 A13,100、P2 **A24,493 / 250 ticks**，B 均24。
- P1 world10 → P2 world28：另一组合代码 hash `81cd576411f67e85c2aa821146613e2db1f10dc5ff51e7c04ad92b12ee428c95`，目标新输入真实第四操作数 **35**；导入29；P1 A16,349、P2 **A24,493 / 250 ticks**，B 均24。两组 C 的实际 SEND 经过协议解码、B 实际结果经独立 FOLD32 核验。
- **严格成功任务反事实**：同一 P2 world19、相同根/导入/起始状态（干预前 state hash 一致），研究者在两臂各以真实 CAS 写入一次**16 字节、3 A gas**的相同对象，唯一区别是翻转来源组合程序第四操作数的最低位。两条 C/B 请求都合法并成功、B均24、A均24,496；真实返回值 **1,534,416,407 → 1,534,416,415（+8）**。这个差异符合第四操作数在 FOLD32 中左旋3位的计算规则。干预是宿主因果试验，不是个体自主修复。
- **错误知识修复**：错误引入 source world10 组合程序到 P2 world19，个体从目标公开标签认出失配、重新搜索和 CAS v0→v1，得正确第四操作数52，B24，A25,259，256 ticks。
- 测试断言了真实程序版本 WRITE、组合工具 EXEC 先于操作数 WRITE、Len EXEC 发生在操作数与数量 WRITE 之间、专家读取两个对象、真实 C on-wire count4+16字节，以及 B 的真实计算与 world snapshot hash 相同，而不只是检测到代码文件或程序哈希。

## 结论限制
这证明**预置有限语法内，经活动 VM 依据公开反馈生成的多节点程序，其输出在跨协议家族后确实成为远程计算的必要操作数，并决定真实 B 结果**。没有证明开放式程序创新、自主演化、任意未知数字服务适配或正式 H3 通过。两个探索性配对与一次干预不足以构成确认性样本。G0 GO；G1/G2 NO-GO。

[活动 VM / 专家代码](../../crates/organism-seeds/src/s32_composite_b_operand.rs) · [原生因果测试](../../crates/organism-seeds/tests/s32_composite_b_operand.rs) · [CI](../../.github/workflows/s32-composite-b-operand.yml)。
