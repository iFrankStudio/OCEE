# DEV — 可信引用证书的有偿批次回收（实现前冻结）

**状态：冻结协议已完成；追加真实红例与最终原生4/4证据。** 基线 main `fe56667d37a2ca85ce1910136672b9ae19161526`，上一真实 [#45121695557](https://github.com/iFrankStudio/OCEE/actions/runs/45121695557) 对 S29 POST_REPAIR 回收 279B 付出 1646 A。本轮不能偷偷把纯VM自身已执行的指令/世界需要付费的扫描换成无偿宿主oracle，也不能把存储释放直接视作净收益。

## 待验证的可证伪机制

不采信 organism-writable `/self/index`。新添 **kernel-owned、带快照持久化的引用证书** `WorldState.version_ref_index`，包含建立时的请求actor、活动根身份、当时已安装全部版本的内容哈希集合、哪些已安装版本在其他世界对象key/raw bytes或pending/mailbox消息中被完整64字符ASCII引用、成功构建前逐字节扫描数量、构建世界step；内核负责证书，个体不可读写伪造。只扫描当时已安装候选hash；新装版本必须使证书失效。旧版本对象中的字符串指针会被索引，但该对象自身key对自身hash的平凡引用排除。与旧GC保持同一**请求者当前活动Program字节码豁免**，这不是一般的未来程序使用安全证明；根、初始恢复root、任何运行中的Continuation哈希仍每次实时检查。

新增有偿VM/世界调用：
1. `IndexVersionRefs()` / `Action::IndexVersionRefs{actor}`：按原旧GC完全相同的对象key+bytes和pending/mailbox消息（包括非目标旧版本的内容）**一次扫描**，收费 `2 + ceil(scanned_bytes/64) + installed_version_count` A；产生核可信引用证书，内核内存索引建立不得依赖一个特定要删除的hash。证书不构成无偿外部正确性oracle。
2. `ReclaimIndexedVersion(hash_bytes)` / `Action::ReclaimIndexedVersion{actor,program_hash}`：仅在证书存在、属本actor、绑定的`active_program_hash`未变化、未被任何可能新增引用的世界写入/消息变化失效时才做快速检查；证书中含真实外部引用或当前root/初始root/continuation引用的仍真实Conflict；安全回收 1 + 2 + ceil(program_bytes/64) A（当前旧GC删除费用+1证书核验费），一次成功删除版本并准确退还其byte存储。对该版本的后续二次回收NotFound。证书可以在**只删除对象的ReclaimIndexedVersion**后继续保守复用：已存在的旧版本对象只会减少引用，不会增加新引用；可能出现false positive但不能出现因本动作形成的false negative。完全扫描旧`ReclaimVersion`不变作为真实对照。

**强制失效/写屏障**：任何传统世界Write/PublishTool、成功SEND/RECEIVE、root重启、动态环境对象发布/更新、服务消息创建/交付/消费、从快照恢复后识别不完整证书、以及其它可能新增对象/消息raw-hash的路径，必须在修改前将证书置None；为避免遗漏，允许因潜在写入尝试导致额外失效。只读Observe/Read/Exec及完成后的纯内部计费不应无故使证书失效。任何证书过期必须`Conflict`，不能静默降级到一次未计费的完整扫描或继续删除。浏览器/测试代码直接改写公开`world.state`不属于真实授权世界动作；伪造裸快照/脱离kernel的内存修改须明确作为威胁模型外的权限。

**经济与CPU界限**：扫描唯一一次，每次收费至少与原旧GC同等按字节计费；构造候选索引时只查当前已安装Program内容hash集合，Hash窗口固定64字节；对扫描窗口的内核CPU工作需限制至每对象字节与版本数量，`installed_version_count`固定增费，不把每次GC扫描搬到未计费的后台。证书本身属于kernel内部受限缓存，占用内存需有可归责的条目上限（按已经受A存储cap约束的真实版本数），核验单版本免费查表的成本以fast动作基础费覆盖。

## 原生矩阵预冻结

- **AMORTIZED**：真实P2 S29 `POST_REPAIR`完成heldout5/B20/正确远程任务，再由研究者安装维护根（明确不是自主生存动机）。在同一完整世界快照上准备 `>=3` 个真正已付费安装、当前无引用的合法旧版本；支付一次`IndexVersionRefs`，用活动VM真正连续发出3个`ReclaimIndexedVersion`动作；并将**完全相同快照**分叉为传统`ReclaimVersion`每次完整扫描。核对版本删除/释放bytes/各次回执、B不重复、总收费和净gas节省；收益只属于批次GC操作，**不把同世界付费准备费用作为净算法优势**。即使成本未下降也必须如实报告。
- **STALE_WRITE**：建立真实有偿证书后，一次合法 `Action::Write` 添加含某旧hash的 /self/pins 引用，证书必须失效；`ReclaimIndexedVersion`返回Conflict且目标版本/存储不变。完整扫描版仍真实拒绝删除；不允许相信伪造用户表。
- **STALE_MESSAGE**：建立真实证书后普通SEND向C放入含旧hash的真实消息（必要时也验证传递到mailbox）、使证书失效；快速回收Conflict，原始扫描仍保护在途消息，不准仅看/ self/tools注册表。
- **GUARDS**：活动/初始根保护、不存在版本/坏hash/非拥有版本、同一证书内多次删除后目标NotFound与全局snapshots保持一致；A不足时应Gas且不能删。除本试验确实必要的关键安全断言外，不加大量冗余测试。

该研究新增的内核索引/2种世界动作都是**研究者设计的元设施**，不等于生命体自己发现GC/形成动机。正式E001旧六原语+前两次开发扩展的版本/成本契约需单独冻结；仍然是**G0 GO、G1/G2 NO-GO，E001 NOT pre-registered**。

## 2026-11-23 — 原生认证引用索引4/4：同快照3版本回收节省3263 A

[最终 GitHub Actions Run #45123571075](https://github.com/iFrankStudio/OCEE/actions/runs/45123571075) @ `382cde319b2f531ae631b9a4a006769f97733dc7` **completed/success，4 独立 Rust matrix jobs 全部 1 passed/0 failed**。定向工作流验收后已改为仅 `workflow_dispatch`，禁止后续push重跑该历史实验。第一轮红例真实留档，不合并报告为“首次4/4”。

### 同一世界快照、真实付费资源账本

实际 P2 world seed129/protocol seed42 S29 原子修复成功，并真实完成 heldout5及独立FOLD32 P2 B计算 **B20**。在**同一S29完成后的付费初始化快照**中，研究者安装真实有效、当前未注册的三个程序版本，并切换到研究者设计的**有界维护VM**：旧`ByteAt(Bytes,1)`哈希 `6820d1219dd6fc5619ffa4244289cc27394e6811149eb7ca81168fe99ec8774b`，其他两个合法版本hash `f0b263f8537295bdaff2afb92bda6527ab91324a3ec26f45f650d6fe70569837`、`91168ca0cb6a15ae4bcc19a8fe104313e9c3aabad7e261658628400566f5d757`，实际字节 `[279,276,279]`。维护根哈希 `2adaec70f8c53c3b4f6df1b17b04f670a75735990d18f788450f80f8709a494e`。前态的完整snapshots `state_hash` 严格相同，先分叉为新`IndexVersionRefs+ReclaimIndexedVersion×3`和旧`ReclaimVersion×3`，独立计数世界Action A，不把先前研究者安装3版本/启动维护根等共同开销算作GC优化。

| 原生核对项 | 旧完整扫描×3 | 认证索引×1+快速回收×3 |
|---|---:|---:|
| 是否实际移除三相同合法版本 | 是 | 是（真实活动 VM WorldCall） |
| **精确归还持久空间** | **834 B** | **834 B** |
| 付费完整扫描的每项Action A | `[1657,1651,1646]` | `[1667]`（含内核已安装条目计费） |
| 新快速回收三项Action A | 无 | `[8,8,8]` |
| **回收动作A总账本** | **4954 A** | **1691 A** |
| **实际节省A** | — | **3263 A，即相对旧模式减少65.9%** |
| 扫描对象和消息输入尺寸 | 每次重扫剩余对象 | **105858 B，仅建立索引时扫一次** |
| 真实先前任务B费用 | 20（没有新任务） | 20（没有新任务） |
| 保存快照恢复后状态哈希 | 一致 | 一致 |

新活动VM得到返回`U32(1)`、完成世界step2066，记录其**整个world累计**VM A17479 + Action A8913（包含S29完整先前任务、3程序有偿导入、维护根切换、证书/3 GC等）。旧路径由研究驱动器在完全相同起始快照下发起外部World Action，因此 `4954 vs1691`是**回收世界动作收费**对照，并**非**两条完整自主VM生命史的等起点净收益、也不是把VM gas与A/B资源混为同一类。只有批次回收才把建索引收费摊薄，回收一个279B版本的收益可为负；本研究不能因此宣称单版本GC已经济合算。

### 三组真实安全负对照

- **STALE_WRITE**：已在世界付费创建证书后，真`Action::Write`写入带原始64位ASCII完整工具hash的`/self/pins`，世界即刻使证书失效，快速GC实际`Conflict`且不删、不退存储；旧完整扫描也`Conflict`。个体可写的`/self/index`假“safe”对象不能伪造内核证书；重新有偿索引后也能检出新增真实引用而`Conflict`。世界状态/快照一致。
- **STALE_MESSAGE**：已建立证书后，真正成功`SEND`进入在途消息的body含原始hash，kernel使证书失效；快速GC`Conflict`且旧完整扫描真实`Conflict`，目标版本未删、存储无变化，快照一致。
- **GUARDS**：实时活动root、初始恢复root在更换active后仍`Conflict`，切换活动根使旧证书失效；坏hash`InvalidCode`、不存在对象`NotFound`，A余额不足`Gas`且无局部删除，snapshot一致。

世界内核本次把 `VersionRefIndex` 作为**kernel-owned、已计费、可序列化到可信世界snapshot**的缓存，而非用户 `/self/` 对象；包含actor/建立时active hash/全部当时已安装Program hash/存在其他活对象或消息原始hash引用的集合/扫描计量/建立step。证书创建先扫描实际物理对象和所有pending/mailbox消息且扣完整扫描费；版本安装、普通Write、Send、Receive、Exec生成公开witness、根重启及环境定时发布/消息转移等可能形成新引用的世界变更，均通过内核失效屏障将证书置None。旧传统`ReclaimVersion`路径**保持不变**。快速GC只对仍存活于证书中的版本、实际actor拥有、完整规范程序且在当下不属于任何活动/continuation/初始恢复root的候选开放；缺证书和新增活引用一律`Conflict`，不会以无偿扫描绕过成本。快速GC成功删除对象只减少可能引用，证书允许保守复用而不允许未认证新增对象。

### 首次红例及真实改动

[首次原生 #45123180187](https://github.com/iFrankStudio/OCEE/actions/runs/45123180187) @`acc3c750da273da891e7c9184da2b1943bcfeb66` **completed/failure，三组安全对照成功，仅AMORTIZED失败**；真实Rust VM报 `InvalidProgram("invalid operand 0")`，因为研究者第二个模拟退役程序把Bytes寄存器0误当作`ByteAt`要求的U32索引。证据表明VM正确拒绝非法测试工具，尚未验证成本。仅在测试夹具把`ByteAt(0,0)`改为`ByteAt(0,1)`（寄存器1原本是`ConstU32(0)`），**没有修改内核引用判定、气费表、seed或评估阈值**；随后[#45123571075](https://github.com/iFrankStudio/OCEE/actions/runs/45123571075)初次完整取得4/4。

### 科学与安全限制

- 仅认证**当前已安装**版本的**原始64字节ASCII完整hash**引用，主动运行维护根的自身字节码按此前GC边界有意豁免，历史audit不视为存活租约；并非全代码未来行为的引用证明，对任意编码变体/多actor隐藏内存引用和分布式外部任务仍有盲点。本项只继承此前保守原始字节索引安全模型；不能以目前4/4宣称任意网络插件环境安全删除。
- Cache通过内核正常世界路径写屏障维护；本项目测试显式修改公开`world.state`的情况不属于授权世界动作，伪造原始snapshot或内存泄露可绕过信任边界。正式使用需补齐封印/鉴权及跨版本迁移策略。
- **开发世界能力已由正式E001的六原语增加到额外 IndexVersionRefs、ReclaimIndexedVersion 等研究设施**，内核/VM/cost table/能力manifest需要版本冻结，不能把这些新能力直接视作个体自主学习能力。研究者既选了GC目标又提供维护VM，不是生命体主动制定维护计划；G0 GO、G1/G2 NO-GO，E001仍 NOT preregistered。
- 下一次优先研究：在**同等输入/同真实运行根**下测量维护root的VM开销与编译/索引CPU成本，求解触发GC的经济阈值、证书失效/刷新频率、在途任务和多actor引用的可信边界；不可只看这次动作A节省就宣布生命体净效用增长。
