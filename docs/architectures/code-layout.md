```text
actrail-kv/
├── Cargo.toml # Cargo workspace 配置
├── crates/ # Rust 库及命令 crate
│   ├── cmd/ # 二进制入口；只解析 CLI 并调用对应 library
│   │   ├── receiver/
│   │   │   ├── src/main.rs # HTTP 接收命令与版本输出
│   │   │   └── examples/generate_config.rs # 从配置类型生成带注释的 TOML 示例
│   │   ├── analyze/
│   │   │   ├── src/
│   │   │   │   ├── main.rs # 执行离线分析并解析运行配置
│   │   │   │   └── selection.rs # 解析报告筛选条件与存储扫描范围
│   │   │   └── examples/generate_config.rs # 从配置类型生成带注释的 TOML 示例
│   │   ├── export/src/ # 导出命令
│   │   │   ├── main.rs # 扫描存储并导出 NDJSON
│   │   │   └── config.rs # ExportSettings
│   │   └── report/src/main.rs # 将 analysis.json 渲染为 HTML
│   │
│   ├── contracts/ # 按消费方和依赖边界组织契约 crate
│   │   ├── analysis/ # 分析执行与报告共同使用的配置契约
│   │   │   ├── Cargo.toml # analysis-contract 独立配置契约包
│   │   │   └── src/
│   │   │       ├── lib.rs # 导出分析配置契约
│   │   │       ├── settings.rs # 分析输入、输出与筛选设置契约
│   │   │       └── config/
│   │   │           ├── mod.rs # AnalyzeConfig 递归组合与校验委托
│   │   │           ├── corpus.rs # CorpusConfig
│   │   │           ├── projection.rs # ProjectionConfig
│   │   │           ├── candidate.rs # CandidateConfig
│   │   │           ├── stability.rs # StabilityConfig，共用稳定性条件
│   │   │           ├── template.rs # TemplateConfig
│   │   │           ├── diagnosis.rs # DiagnosisConfig
│   │   │           └── prefix_view.rs # PrefixViewConfig
│   │   ├── report/ # 独立报告配置契约
│   │   │   ├── Cargo.toml # report-contract，仅依赖通用配置契约和 Serde
│   │   │   └── src/
│   │   │       ├── lib.rs # 导出报告配置与格式
│   │   │       ├── settings.rs # ReportSettings 字段与默认值
│   │   │       └── format.rs # ReportFormat 及格式名称解析
│   │   ├── storage/src/ # SDK 与存储实现共用的配置契约
│   │   │   ├── lib.rs # 存储配置、覆盖类型、路径解析与配置错误
│   │   │   ├── backends.rs # SQLite 与 NDJSON 配置和保留期
│   │   │   ├── defaults.rs # 默认保留 7 天及 256 MiB 容量上限
│   │   │   └── resolved.rs # 后端选择、保留期计算与输出路径保护
│   │   └── config/
│   │       ├── Cargo.toml # TOML 配置契约的独立包
│   │       ├── derive/ # Config 派生宏包
│   │       │   ├── Cargo.toml
│   │       │   └── src/
│   │       │       ├── lib.rs # 生成 Config 实现并提取字段注释
│   │       │       ├── deserialize.rs # 生成基于 Serde 的业务类型解析
│   │       │       ├── fields.rs # 从 Serde 元数据生成字段识别与别名对齐
│   │       │       └── overrides.rs # 从配置字段生成可选覆盖结构体
│   │       └── src/
│   │           ├── lib.rs # 导出配置接口、派生宏和 TOML Table
│   │           ├── config.rs # Config、RootConfig、备份和原子写回
│   │           ├── overrides.rs # 嵌套配置与叶子字段的覆盖类型映射
│   │           └── derive_support/
│   │               ├── mod.rs # 连接 Serde 与 TOML 文档模型
│   │               ├── comments.rs # 在已序列化的键和表上递归附加注释
│   │               ├── fields.rs # 递归识别配置字段、对齐别名并收集未知字段路径
│   │               ├── completion.rs # 补齐缺失字段并保留已有值与注释
│   │               └── state.rs # 检测需要写回的缺省字段
│   │
│   ├── artifacts/src/ # 命令共享的采集记录和分析产物协议
│   │   ├── captured_request.rs # CapturedRequest、显式 ComparisonMetadata
│   │   ├── analysis_result.rs # AnalysisResult、template、统一 ContextDefect、Top K ID
│   │   ├── analysis_result/
│   │   │   ├── session.rs # 会话刷新、逐请求前缀复用度与事件协议
│   │   │   ├── prefix_reuse.rs # 结构复用率与可恢复上界协议
│   │   │   ├── cache_metric.rs # 逐请求缓存命中率协议
│   │   │   └── run_summary.rs # 运行摘要、参数快照与 skip 协议
│   │   └── lib.rs # 公共 DTO re-export
│   │
│   ├── receiver/src/ # 持久采集服务与 HTTP 通道，不依赖 analyzer
│   │   ├── config.rs # HTTP 配置字段、默认值和校验
│   │   ├── config/settings.rs # CLI 文件配置入口
│   │   ├── contracts/mod.rs [x] # CaptureRequest、已校验 CaptureInput、回执、错误与 CaptureFactory
│   │   ├── service/mod.rs [x] # 生成采集时间并等待后端写入
│   │   ├── channels/ [x] # 输入通道适配
│   │   │   ├── mod.rs # 通道模块入口
│   │   │   └── http/mod.rs # HTTP 解码、header 提取、字节限制与响应映射
│   │   └── lib.rs # 公共 API 与兼容路由入口
│   ├── sdk/ [x] # 0.2.0 C ABI 动态交付与 Rust 安全调用
│   │   ├── native/ # 原生动态库及公共边界支持
│   │   │   ├── contracts/ # 固定 C 布局、版本与响应信封
│   │   │   │   ├── src/lib.rs # Buffer、函数签名、Response 与 ErrorDetail
│   │   │   │   └── include/actrail.h # actrail_kv_ C/C++ 公开函数和内存约定
│   │   │   ├── runtime/src/ # 公共原生执行和资源管理
│   │   │   │   ├── lib.rs # 版本化 JSON 请求入口
│   │   │   │   ├── boundary.rs # 指针、句柄、响应分配与 panic 边界
│   │   │   │   ├── worker.rs # 专用线程、实例与 Tokio 生命周期
│   │   │   │   ├── config.rs # 文件/内存配置、字段覆盖、补全与路径基准
│   │   │   │   ├── error.rs # 稳定状态码和错误详情
│   │   │   │   ├── exports.rs # 共用 C ABI 生命周期符号
│   │   │   │   ├── source.rs # 采集存储和文件来源保护
│   │   │   │   └── output.rs # 显式原子导出
│   │   │   ├── receiver/src/lib.rs # 公共采集服务的 C ABI 门面
│   │   │   ├── analyzer/src/ # 存储筛选、内存分析与显式导出
│   │   │   │   ├── lib.rs # 原生分析入口与来源记录
│   │   │   │   └── selection.rs # 按次配置覆盖、粗筛和精筛转换
│   │   │   └── reporter/src/lib.rs # 直接渲染、实例语言及报告导出
│   │   └── rust/ # 不链接业务核心的安全 SDK
│   │       ├── src/
│   │       │   ├── lib.rs # 对外模块和类型重导出
│   │       │   ├── contracts/ # SDK 公开配置契约
│   │       │   │   ├── mod.rs # SdkConfig、SdkError
│   │       │   │   └── overrides.rs # SdkOverrides 与生成的强类型字段覆盖
│   │       │   ├── receiver/contracts/mod.rs # 持久采集类型与 ReceiverFactory
│   │       │   ├── analyzer/contracts/mod.rs # 筛选、AnalysisOutput 与 AnalyzerFactory
│   │       │   ├── reporter/contracts/mod.rs # 渲染、ReportOutput 与 ReporterFactory
│   │       │   └── loader/ # ABI 加载和所有权
│   │       │       ├── mod.rs # 动态符号与版本检查
│   │       │       ├── instance.rs # 独占实例和显式关闭
│   │       │       └── output.rs # 原始响应借用、释放及显式导出
│   │       └── examples/ # 可运行宿主示例
│   │           ├── sdk_pipeline.rs # 带默认文件配置、筛选、存储和输出说明的完整调用
│   │           └── sdk_probe.rs # 独立动态库升级验证宿主
│   ├── settings/src/ # 共享配置文件访问
│   │   ├── lib.rs # ConfigFile 按 locator 读取、锁定并写回
│   │   └── document.rs # 补齐缺项并保留用户值和注释
│   │
│   ├── storage/src/ # 采集存储接口、后端配置和扫描筛选
│   │   ├── lib.rs # CaptureStore、CaptureStream 和 StorageError
│   │   ├── config.rs # 重导出独立存储配置契约
│   │   ├── filter.rs # session 等值和 UTC 半开区间
│   │   ├── runtime.rs # 有背压流式扫描与周期清理
│   │   └── backends/
│   │       ├── mod.rs # 工厂与只读、可写入口
│   │       ├── sqlite/ # SQLite 事务、扫描和容量回收
│   │       │   ├── mod.rs # WAL、索引查询、快照与到期清理
│   │       │   └── capacity.rs # JSON 字节计数、迁移与最旧记录淘汰
│   │       └── ndjson/
│   │           ├── mod.rs # 写入、轮转、读取与回收
│   │           ├── capacity.rs # 分段总字节计量与最旧分段淘汰
│   │           ├── segments.rs # 分片名称、崩溃尾部修复和有界读取
│   │           └── appender.rs # 并发安全、整行追加、独占与权限
│   │
│   ├── analyzer/src/ # 模板抽取与结构诊断核心
│   │   ├── cache_metrics.rs # 每请求 KV 命中率：上游 usage 优先，缺失时按公共前缀估算
│   │   ├── config.rs # 导出共享分析配置与业务入口设置
│   │   ├── prefix_view.rs # 每模板前缀视图模型：稳定/变体片段、变体、缺陷关联
│   │   ├── model/
│   │   │   ├── comparison/
│   │   │   │   ├── group_key.rs # 时间窗口+endpoint+model/deployment+schema+agent+namespace
│   │   │   │   └── mod.rs
│   │   │   ├── corpus/
│   │   │   │   ├── loader.rs # 有界读取 NDJSON、坏记录 skip、稳定请求 ID
│   │   │   │   ├── selection.rs # session、时间并集和最近记录选择
│   │   │   │   ├── types.rs # CorpusRecord、CaptureComparison、skip 类型
│   │   │   │   └── mod.rs
│   │   │   ├── projection/
│   │   │   │   ├── projector.rs # 投影 tools/messages/content blocks 的模型可见顺序
│   │   │   │   ├── types.rs # CacheSequence、CacheUnit、HierarchyLocation、source
│   │   │   │   └── mod.rs
│   │   │   └── mod.rs
│   │   │
│   │   ├── discovery/
│   │   │   ├── candidate/
│   │   │   │   ├── signature.rs # 有界结构兼容度与候选召回签名
│   │   │   │   ├── cohort.rs # 确定性模板 cohort、bridge 防护和预算
│   │   │   │   └── mod.rs
│   │   │   ├── template/
│   │   │   │   ├── align.rs # 有界 unit sequence alignment
│   │   │   │   ├── extractor.rs # stable span、slot、模板坐标与成员映射
│   │   │   │   ├── types.rs # RequestTemplate 及抽取结果
│   │   │   │   ├── symbolic/
│   │   │   │   │   ├── coordinate.rs # TemplateCoordinate、逻辑单元 key
│   │   │   │   │   ├── member_map.rs # 每成员 coordinate binding、Gap、UnmatchedRun
│   │   │   │   │   ├── sequence.rs # 一等 SymbolicMemberSequence 与 SymbolicAtom
│   │   │   │   │   ├── build.rs # 从 alignment 构造成员坐标与符号序列
│   │   │   │   │   ├── identity.rs # 不依赖成员/绝对 unit index 的坐标 ID
│   │   │   │   │   └── mod.rs
│   │   │   │   └── mod.rs
│   │   │   └── mod.rs
│   │   │
│   │   ├── diagnosis/
│   │   │   ├── episode/
│   │   │   │   ├── locator.rs # 从全部符号成员定位唯一 P1/X/P2 region
│   │   │   │   ├── variant.rs # 每成员一次的 X 变体和 P2 证据构造
│   │   │   │   ├── model.rs # DefectCandidate、EpisodeVariant 与诊断配置契约导出
│   │   │   │   └── mod.rs
│   │   │   ├── facts/
│   │   │   │   ├── content.rs # 内容变化和少数固定变体事实
│   │   │   │   ├── representation.rs # 模型可见 JSON 严格等价事实
│   │   │   │   ├── sequence.rs # 插入/缺失及唯一身份重排事实
│   │   │   │   └── mod.rs # 合并 facts/insights，不拆分同一 episode
│   │   │   ├── identity/mod.rs # 基于逻辑 region 与完整 P2 anchor 的 defect ID
│   │   │   └── mod.rs
│   │   │
│   │   ├── ranking/
│   │   │   └── mod.rs # 已唯一化 candidate 的评分、稳定全排序、Top K
│   │   ├── session/
│   │   │   ├── mod.rs # 会话 prefix-switch 与结构复用率模块入口
│   │   │   ├── lineage.rs # 会话时间线、上下文刷新与相邻对分类
│   │   │   └── reuse.rs # 结构复用率汇总与可恢复上界
│   │   ├── run/
│   │   │   ├── pipeline.rs # 内存分析计算、artifact 映射与 CLI 原子输出包装
│   │   │   └── mod.rs
│   │   └── lib.rs
│   │
│   └── reporter/ # 读取 analysis.json 并渲染报告
│       ├── src/
│       │   ├── config.rs # 重导出报告配置契约
│       │   ├── result_loader.rs # 内存/文件统一加载和 schema、引用、P2、score 校验
│       │   ├── html_report_renderer/
│       │   │   ├── mod.rs # Askama 根模板渲染入口
│       │   │   ├── template.rs # 编译期嵌入的 HTML 根模板
│       │   │   ├── format.rs # 字节、短 ID、摘要与位置格式化
│       │   │   └── view/ # HTML 展示模型与各区块数据装配
│       │   │       ├── mod.rs # HtmlReportView 根模型
│       │   │       ├── defects.rs # Top K 与缺陷详情
│       │   │       ├── prefix_reuse.rs # 复用率与真实命中对照
│       │   │       ├── session.rs # 会话刷新、逐轮与事件
│       │   │       ├── cache.rs # KV 缓存指标
│       │   │       └── prefix_tree.rs # 前缀树、成员衰减与 SVG 坐标
│       │   ├── locale/ # 全局 locale 工厂、快照与 namespace 子表
│       │   │   ├── mod.rs # Locale、namespace 常量与公共 API
│       │   │   ├── factory.rs # 全局语言兼容入口与独立实例语言快照
│       │   │   ├── registry.rs # locale/namespace/key 注册与完整校验
│       │   │   ├── snapshot.rs # 不可变快照、模块句柄与 format
│       │   │   └── catalogs/ # zh-CN / en-US 文案子表
│       │   ├── markdown_report_renderer/ # 纯 GFM 报告，与 HTML 信息内容等价
│       │   │   ├── mod.rs # Markdown 总章节编排
│       │   │   ├── format.rs # GFM 转义、表格与代码片段辅助
│       │   │   ├── prefix_reuse.rs # 结构复用率与真实命中对照
│       │   │   ├── defects.rs # Top K 与缺陷详情
│       │   │   ├── session.rs # 会话上下文刷新
│       │   │   ├── cache.rs # KV 缓存指标
│       │   │   └── prefix_tree.rs # 模板前缀树
│       │   ├── report_output.rs # 格式选择、路径派生与双产物原子发布
│       │   └── lib.rs
│       └── templates/report/ # 编译期嵌入的 HTML、CSS 与区块 partial
│           ├── layout.html # 文档头、CSP、全局编排
│           ├── styles.css # 全部静态报告样式
│           └── sections/ # Top K、缺陷、会话、缓存与前缀树模板
│
├── examples/
│   ├── quickstart/
│   │   └── requests.ndjson # 根 README 可直接分析的最小缺陷语料
│   ├── cases/ # 真实语料回归 fixture（workspace/policy/json/insert/natural/session/context-refresh）
│   ├── analyze.config.example.toml # analyze 全字段 TOML 配置示例
│   └── receiver.config.example.toml # receiver TOML 配置示例
│
└── tests/
    ├── unit/ # 按 crate 和模块归档的单元测试
    ├── integration/ # 由所属 crate 清单注册的集成测试
    │   ├── analyzer/ # 语料回归、验收、属性和规模测试
    │   ├── contracts/
    │   │   ├── config.rs # 手写配置契约与写回行为
    │   │   └── config/derive.rs # 业务配置派生、递归注释和读写行为
    │   ├── sdk/ # 原生 SDK 集成测试
    │   │   ├── default_settings.rs # 隔离用户目录验证默认文件、字段覆盖和导出路径
    │   │   ├── native.rs # 真实动态库持久采集、内存交接与输出生命周期
    │   │   └── worker_lifecycle.rs # panic 隔离和后台运行时关闭
    │   ├── settings/ # 并发补全和保留用户值
    │   └── storage/ # 后端选择、轮转、筛选和清理
    │       ├── backends.rs # 时间保留期、扫描快照及轮转恢复
    │       └── capacity.rs # 容量淘汰、超限拒写、后端覆盖与既有库迁移
    ├── fixtures/sdk/abi_fixture.c # 按公开头文件实现的兼容升级测试库
    └── end_to_end/
        ├── sdk_upgrade.sh # 同一宿主替换库、ABI 与符号拒绝
        ├── shared_settings.sh # 共享设置和默认路径的完整流程
        └── three_binaries.sh # 真实 receiver→analyze→report、确定性与 XSS 验收
```

状态：`[ ]` 待开始，`[/]` 修改中，`[x]` 完成，`[-]` 待删除或待迁移走。无标记节点为现有代码。
