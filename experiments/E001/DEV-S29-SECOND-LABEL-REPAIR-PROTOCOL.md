# DEV — S29 第二个公开样例才暴露的错误继承工具：真实VM自修复协议（冻结）

**状态：运行前冻结，尚无本项原生结果。** 从 OCEE `main` 基线 `507d9414b1c252edf223f6c8825304d03187d435` 继续；上阶段 [S29首样例反馈修复原生4/4](https://github.com/iFrankStudio/OCEE/actions/runs/45107700394) 已证明首次样例失败时可以有限修复、付费CAS采用，陈旧注册表则拒绝覆盖并残留未注册程序。

## 待排除的“看似可信继承”问题

旧S29根校验继承工具时要求公开双例匹配，但上阶段只能在**第一例**结果冲突时进入重新选择路径：若第一例正确而第二例错误，根仍会U32(0)停止。这不是“没有足够数据”的问题，公开第二例标签已经存在，缺的是第二个分支上的自主修复与有界控制流。不能从第一例通过推断工具完整正确。

目标P2协议seed42、world seed129，公开输入首例 `[32,3,8]`→`Len=3`、次例 `[18,2,7,4]`→`Len=4`；held-out `[32,0,9,6,2]` 的正确答案5只在测试外部核验，活动VM不得读取隐藏答案。研究者安装一个**合法纯VM但故意仅在第一例相符**的对抗工具：`ByteAt(Bytes,1)` 对第一例真实返回3，对第二例真实返回2，公开标签4；它并非过去P1真实生出的donor，需单独披露。正常Len与错误XorFold则从P1来源个体的实际生成程序合法付费导入。

## 冻结实现约束

- 继承S29原根的全部公开两标注例、三个研究者提供原语 `Len/XorFold/ByteAt0`、`RewriteOp`、内容寻址版本、有偿 `Action::Exec`、付费CAS `expected_version=0`，不引入世界专用答案或VM特权oracle。
- 初次继承工具真EXEC第一样例正确才继续真EXEC第二样例；第二例反馈 **Code::Ok但 U32不等于公开标签** 时，VM自身再次有偿Read公开 `/public/E001/transform`，解析两例标签，进入原候选选择/生成/付费版本安装/registry CAS路径。
- 必须避免 `unbounded control-flow cycle`：原初次第二例校验失败进入恢复，修复后**采用另一个单向的第二例校验块**，它无论成败都不能回到第二次恢复/候选搜索。初次第一例失败、原首例修复机制应仍能工作。严格MAX_BLOCKS和MAX_INSTRUCTIONS不变。
- CAS0后如注册表不存在，最多一次None创建；若注册表已为version1，CAS0与None均冲突，必须安全退出、禁止错误预测/B操作。现内核没有Delete，也没有原子双键CAS，把候选版本先安装再更新注册表会留下孤儿版本，不能声称已回滚或内核事务性修复。
- world seed129, P2 protocol seed42, A12,000/epoch, 16,384调度tick限，不能调大以追求绿色；根程序一致，研究者插入的旧工具不同，跨臂A总费用不等于匹配因果净收益。

## 四臂定向原生判据

| 臂 | 活动前注册状态 | 应有真实行为 |
|---|---|---|
| MATCHED | 合法P1 Len（version0） | 原工具对两例通过；不新增可执行版本；3次Exec包括held-out；真实B20，正确落盘 |
| FIRST_MISMATCH | 合法P1 XorFold（version0） | 首例真实Exec43 vs label3→活动VM重选Len→CAS0 Ok→新Len真Exec 3次→B20 |
| SECOND_MISMATCH | 研究者有偿安装的 ByteAt(bytes,1)（version0），首例碰撞 | 首例真Exec3/label3；次例真Exec2/label4；再有偿Read公开完整双标签→VM重选Len→CAS0 Ok→新Len真Exec三次并正确保存held-out及B20 |
| SECOND_STALE | 同样对抗工具，但研究者前置合法CAS0先升为version1（值不变） | 真Exec两个样例后复选Len并安装有偿版本；CAS0 Conflict+None Conflict，旧version1工具不变、无held-out预测/B0/远端结果，候选版本对象可能留存 |

原生测试必须分别审计两个旧工具Exec的输入、返回值和有偿gas，新Len生成/Exec、目标自主开始后的版本Write次数、registry CAS反馈及最终版本、独立 `hidden_transform` 仅在实验后的真值核对、真实B gas/结果、A_VM/A_action、世界steps、VM无非法动作、无越权重启和snapshot一致。成功组的B20只能表示原生P2工作实际发生，仍不是正式跨家族未知协议学习的证明。

此仍为**研究者给出的有限搜索空间与修复图**，并非自主发明全新原语、可信锚自主更新、开放式演化或正式E001预注册；G0 GO、G1/G2 NO-GO。保持原生首次红绿过程与实际失败结果，不为绿色改动资源和公开标签。
