```text
actrail-kv/
├── Cargo.toml # Cargo workspace 配置
├── crates/ # Rust 库及命令 crate
│   ├── cmd/ # 二进制入口；只解析 CLI 并调用对应 library
│   │   ├── receiver/
│   │   │   ├── src/main.rs # 启动 HTTP receiver
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
│   │   └── config/
│   │       ├── Cargo.toml # TOML 配置契约的独立包
│   │       ├── derive/ # Config 派生宏包
│   │       │   ├── Cargo.toml
│   │       │   └── src/
│   │       │       ├── lib.rs # 生成 Config 实现并提取字段注释
│   │       │       ├── deserialize.rs # 生成基于 Serde 的业务类型解析
│   │       │       └── fields.rs # 从 Serde 元数据生成字段识别与别名对齐
│   │       └── src/
│   │           ├── lib.rs # 导出配置接口、派生宏和 TOML Table
│   │           ├── config.rs # Config、RootConfig、备份和原子写回
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
│   ├── receiver/src/ # 最小接收端；不依赖 analyzer
│   │   ├── config.rs # ReceiverConfig 字段、默认值和校验
│   │   ├── config/settings.rs # 业务入口配置与嵌套设置
│   │   ├── http_receiver.rs # POST /requests、metadata headers、captured_at
│   │   └── lib.rs # receiver library API
│   ├── settings/src/ # 共享配置文件访问
│   │   ├── lib.rs # ConfigFile 按 locator 读取、锁定并写回
│   │   └── document.rs # 补齐缺项并保留用户值和注释
│   │
│   ├── storage/src/ # 采集存储接口、后端配置和扫描筛选
│   │   ├── lib.rs # CaptureStore、CaptureStream 和 StorageError
│   │   ├── config/ # 存储配置及后端解析
│   │   │   ├── mod.rs # StorageConfig、后端类别与路径解析
│   │   │   ├── backends.rs # SQLite 和 NDJSON 配置及保留期
│   │   │   ├── defaults.rs # 存储默认值
│   │   │   └── resolved.rs # 选中后端配置、保留期计算与输出路径保护
│   │   ├── filter.rs # session 等值和 UTC 半开区间
│   │   ├── runtime.rs # 有背压流式扫描与周期清理
│   │   └── backends/
│   │       ├── mod.rs # 工厂与只读、可写入口
│   │       ├── sqlite.rs # WAL、索引查询和到期清理
│   │       └── ndjson/
│   │           ├── mod.rs # 写入、轮转、读取与回收
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
│   │   │   ├── pipeline.rs # 文件/reader 编排、artifact 映射与原子输出
│   │   │   └── mod.rs
│   │   └── lib.rs
│   │
│   └── reporter/ # 读取 analysis.json 并渲染报告
│       ├── src/
│       │   ├── config.rs # 重导出报告配置契约
│       │   ├── result_loader.rs # schema、引用、成员、P2、score、Top K 强校验
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
│       │   │   ├── factory.rs # 初始化、切换和当前 locale 快照
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
    │   ├── settings/ # 并发补全和保留用户值
    │   └── storage/ # 后端选择、轮转、筛选和清理
    └── end_to_end/
        ├── shared_settings.sh # 共享设置和默认路径的完整流程
        └── three_binaries.sh # 真实 receiver→analyze→report、确定性与 XSS 验收
```
