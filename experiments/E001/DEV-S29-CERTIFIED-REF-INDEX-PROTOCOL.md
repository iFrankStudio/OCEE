# DEV — 可信引用证书的有偿批次回收（实现前冻结）

**状态：先冻结实验；尚未实现与运行原生验证。** 基线 main `fe56667d37a2ca85ce1910136672b9ae19161526`，上一真实 [#45121695557](https://github.com/iFrankStudio/OCEE/actions/runs/45121695557) 对 S29 POST_REPAIR 回收 279B 付出 1646 A。本轮不能偷偷把纯VM自身已执行的指令/世界需要付费的扫描换成无偿宿主oracle，也不能把存储释放直接视作净收益。

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
