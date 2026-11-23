# DEV — S29 有偿自身版本目录与受限VM候选发现（实施前冻结）

**状态：先冻结协议；原生四臂第一次4/4全部通过，结果归档。** 起点main `7557be27e3ca44750ddcd60808ec9c9569b70dd9`。前一[原生6/6逐对象引用差分](./DEV-S29-PAID-SOURCE-DELTA-CERTIFICATE-PROTOCOL.md)验证安全回收工具，但研究者仍直接将目标程序的64位哈希写入活动维护VM的程序字节码；这不是个体发现候选。此轮唯一新增环境能力为普通受费`Read`系统元数据路由，**不增加新的VM/Action原语**，不改变GC正确性/费用或版本引用证书语义。研究者仍负责一次合法主动激活维护根，不能称为内生苏醒或目标发明。

## 新观测接口边界与可证伪设计

`Read("/sys/self-version-catalog/<page>")`，`page`是无前导零的规范十进制u32，实际限制0..=127，每页最多8个条目，**最多1024个**本actor真实`/self/versions/<64位hex>`程序对象，按`(程序实际存储字节长度升序, hash字典序)`稳定排序。结果是JSON `{"page":p,"count":n,"total":N,"entries":[{"hash":"...","bytes":size},...]}`，不带其他actor对象、其他`/self/`数据字节、隐藏环境seed、可回收/被引用分类、安全oracle或任何内部受保护路径；根、initial和注册版本**不排除**，使它们在VM真正调用有偿`ReclaimIndexedVersion`时得到真实`Conflict`。长度/owner/hash格式不符时不得强行猜测。无有效目录页可以返回count0；目录总数>1024、格式非法或数值溢出分别`Quota`/`BadRequest`，不得泄露部分结果。真实每次读取收费 `2+ceil((全部可见版本key字节数+本次结果payload字节数)/64)` A，先计算再扣，Gas拒绝时无payload，不刷新索引或篡改存储。目录只是**所有者资源元数据**，不是已鉴定可回收列表。

固定、研究者预装的有限VM图（相同bytecode跨不同世界）：先`WorldCall::Read`目录第0页，从真实ActionResult反馈中解析entries/count（最多8），然后`WorldCall::IndexVersionRefs`真实有偿建立证书；以`RepeatBounded(limit=8)`逐条动态从目录中提取hash、发起`WorldCall::ReclaimIndexedVersion`，只把`Conflict`/`NotFound`当成换下个候选的正当理由，碰到`Gas`/其它失败直接停止；第一笔Ok即返回1，扫描耗尽返回0。**bytecode中不得出现候选的64位hash literal**；从返回的目录bytes解码才允许真实行动。它实现的是**受限自主候选发现与尝试**，不是个体发明维护目标、证明最优策略、无限遍历所有目录、开放式演化、净资源收益或跨机器能力。目录只第一页，后续分页能力可用但此固定VM不自动翻页。

## 冻结的原生验证四臂

1. `CATALOG_CONTRACT`：基于真实S29 P2已修复world129/protocol42与已有版本对象，调用普通`Read`合法页0；逐条以直接世界状态事实核验owner、key/hash、byte size、排序以及total与count。费用真正>0，重复目录读取按同费扣A且state audit可重放；合法页出界count0、非法`00`/非数字/过大返回BadRequest，不出现跨actor或匿名private字节；Gas不足必须返回Gas/0且不改变版本对象、活索引、存储。B仍20。
2. `DYNAMIC_RECOVERY`：S29 P2真实已修复世界先由研究者真实安装三个未使用版本 `279+276+279B`（哈希只在fixture中用于核查，绝不放入维护VM字节码）；研究者一次CAS激活**同一固定catalog维护root**，内存压力容量设为当前used+2048。活动VM真实读取目录、实际执行一次有偿Index，然后按目录升序发起多个真实GC，保护当前active/initial/registered，至少一个实际成功回收，且audit中每个目标必须来自此前带有实际A费的目录响应；P2 B20不重复、完整snapshot replay一致；打印操作数、成本、回收量、catalog/索引费用及root hash，不声称其候选由研究者植入bytecode。
3. `PIN_AND_CONTINUATION`：在与上一arm**同一root bytecode**的世界中，研究者对第0页最先可回收的原始完整hash加一个**真实付费长pin**，在VM读完目录+真Index后中断到snapshot克隆并继续运行。GC对于该pin目标必须`Conflict`且对象不删；VM只能有界尝试其它候选或返回0，绝不能跳过内核安全。两份恢复世界的完整audit与snapshot必须相等；所有费用均由真实世界动作/VM切片记录。若第0页实际没有可回收目标，必须报告此上限，不能暗中给VM传入一个特定hash。
4. `INVALIDATED_CERT`：相同VM在真正收费Index之后，由研究者执行一个引入实际64B原始hash引用的Write（新普通`/self/pins/`路径），必须像旧机制一样使索引失效；后续`ReclaimIndexedVersion`一律真实Conflict、不删除任何版本，VM无法依靠目录自称安全绕过证书失效。不能重写GC/扫描规则；原S29 P2 B20、root单次活动、snapshot一致。

## 执行、资源、失败约束

每个世界原生Rust独立arm；不得改seed129、P2协议42、A12000每epoch、已有工具语义、GC原始64B安全或旧研究代码/准则。必要修复若出现红run，记录原run和只修编译/fixture本身错误，不调整科学阈值。新路由的读取真实按世界对象数量与响应长度收费，扫描不可伪装成免费sys接口。目录后新增或删除对象可以使候选快照陈旧；只有**实时内核GC**能决定是否回收。实验记录实际VM动作、A/B、存储释放、索引构建次数，明确新增库存目录是研究者赠予个体的观察能力。成功后专项workflow恢复`workflow_dispatch` only。

**研究闸门仍为 G0 GO / G1、G2 NO-GO；正式E001还未校准/预注册。**


## 2026-11-23 — 首轮原生4/4（独立Ubuntu Rust）验收

**[Actions #45180137615](https://github.com/iFrankStudio/OCEE/actions/runs/45180137615)** @ `b905f6bdd02ea65e94aba635c5e02fe1c31bec5b`，4个独立 job 全部 `completed/success`，每臂`1 passed/0 failed`，首次无编译/机制红例。新公开能力实际由 `crates/world-kernel/src/lib.rs` 的旧 `Action::Read` 分支提供，范围仅自身真实`/self/versions/<64hex>`物理存储元数据，最多1024项，每页8项，按`(bytes,hash)`确定性排序；没有改变VM语言语义和GC处理。所有4组活动维护根为**同一个、无待删程序hash常量的** `49d4f015181b1773b0ae77e38d53ba90c89ef85e985d8c163e7dedd7fb1c2036`。共用真实S29 P2 world129/protocol42，前置真实业务已正确完成，B费20，a_gas_per_epoch12000，真实付费研究者安装待删3版本834B，活动维护根由研究者做一次真实读版本+CAS激活。

| Arm | 原生目录及真实Action审计 | VM A（维护片段） | Action A（维护片段） | 真实GC结果 | 回收字节 |
|---|---|---:|---:|---|---:|
| `CATALOG_CONTRACT` | 自有版本10项；页0有偿**27A**；重复读取同响应/同费、页127 count0；非法00/非数字/128返回BadRequest；研究者耗尽gas后真Gas/0A且对象/索引不变 | — | — | 未尝试 | 0B |
| `DYNAMIC_RECOVERY` | 活动VM真实Read页0 **27A**，再真`IndexVersionRefs`一次 | **700** | **1702** | `Conflict, Conflict, Ok(8A)`，三个动态hash均来自实际有偿目录 | **276B** |
| `PIN_AND_CONTINUATION` | 研究者额外实际写入一个完整64B活pin，活动VM读目录/建索引后用**真实snapshot分叉**两次继续，最终snapshot及完整audit相同 | **788** | **1709**（包含研究者pin） | `Conflict,Conflict,Conflict,Ok(8A)`，被pin对象不删 | **279B** |
| `INVALIDATED_CERT` | VM实际完成目录+真Index后，研究者有偿写入64B真hash令证书失效；不改内核GC以服务VM | **1156** | **1700**（包含研究者pin） | 第0页8个候选均真实`Conflict/0A`，原版本无任何误删 | **0B** |

原生目录契约还核对每项hash与当前world实际对象**owner/版本key/物理字节长度**、排序、total、count一致；结果中不提供`referenced`、`protected`或“建议回收”字段；每次目录扫描成本按全部可见版本key长度+所返回JSON payload长度收费，不是假装免费枚举。唯一允许研究者直接改世界内部状态的地方是`CATALOG_CONTRACT`的**明确负例gas枯竭注入**，没有把它冒充付费生命行为。全部四臂全程保留原P2业务B20、单次根激活、后续不变更工具注册表，真实world快照恢复后state_hash一致。

### 科学解读

1. **首次真正消除候选身份硬编码**：同一个固定VM图没有任何三个旧版本的hash字面常量；单次合法启动后通过`Read`付费观察实时拥有的版本目录，`JsonField...`/`JsonIndex`动态取得目标，`RepeatBounded(8)`在失败时使用**真实内核**的Conflict/NotFound反馈继续，找到首个能够真实删除的旧程序。观察、索引、GC、VM消耗均可从不可更改的世界action/audit重算。这是有限**候选自主发现/选择**，不是策略自主涌现。
2. **更一般的引用安全没有被跳过**：catalog有意包含活工具、活动root、初始root甚至被pin对象，VM不能从目录得知是否安全。PIN负例真实拒绝被pin对象但继续找到另一候选，实际snapshot克隆后两份完整audit和状态逐字节一致。INVALIDATED_CERT展示观测已完成也不能绕过后续证书失效，全部8次真实GC都被拒绝。
3. **关键资源代价**：DYNAMIC单次只收回276B，却耗费维护区间VM **700A** +世界Action **1702A** = **2402 A**，其中目录本身真实27A、完整证书构建仍是高额主要成本。它较之前`固定3版834B/1888A`维护策略更弱的资源效率，但两者根与世界状态并不完全同起点，**不能当严格配对性能回归**。这是一项能力边界扩展，不是资源效率突破；费用很可能在有限任务上超过回收价值（A与存储字节不是可直接相减的同单位资源）。
4. **重要限制**：目录首8项，依实际程序大小和哈希稳定顺序；若所有项都被引用，VM返回0，不会遍历第二页；选的是“首个可回收”的小候选，而非最大能释放空间的候选。搜索策略与元数据观察通道仍**研究者提供**；外部研究者还进行了1次root激活和旧候选安装。没有自动触发、未知资源修复策略自创、策略长期保留/进化、开放式演化证据。索引扫描与证书仍仅认原始64B小写hash，外部编码引用模型未证明。

## 决策与后续实验要点

最值得研究下一阶段不是添加大量重复“只测GC安全”的测试，而是研究**真实受费候选评估/净效用控制**：个体能否在先读取实际资源状态后，自行选择何时值得支付目录/索引/VM费用；以及分页下的非首条候选、过期目录视图、已安装版本增长时的最坏费用。必须冻结同一真实世界对照、保留未命中和失败控制，不能直接把目录排序标签当“oracle”。

**仍为G0 GO / G1、G2 NO-GO；正式E001未校准/预注册。** 本轮只证明专项原生4/4，不代表整个Rust workspace全量重新跑通。专项workflow归档后恢复仅手动运行。
