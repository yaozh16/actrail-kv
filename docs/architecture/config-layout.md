| 配置项 | 外部数据类型 | 内部类型 | 默认值 | 必填 | 所属模块 | 说明 |
| --- | --- | --- | --- | --- | --- | --- |
| `receiver` | `table` | `ReceiverSettings` | 各字段默认值 | 否 | receiver | HTTP 接收器设置 |
| `receiver.listen` | `string` | `String` | `"127.0.0.1:8080"` | 否 | receiver | HTTP 监听地址，必须为有效套接字地址，可由 --listen 覆盖 |
| `receiver.max_payload_bytes` | `integer` | `usize` | `8388608` | 否 | receiver | 单次请求体字节上限，须大于零，可由 --max-payload-bytes 覆盖 |
| `receiver.output` | `string` | `Option<PathBuf>` | 无 | 否 | receiver | 设置后使用单文件 NDJSON 输出，否则使用 storage，--output 可覆盖 |
| `storage` | `table` | `StorageConfig` | SQLite 后端及各字段默认值 | 否 | storage | Receiver、Analyzer、Export 共用的采集存储设置 |
| `storage.backend` | `string` | `StorageBackendKind`（enum） | `"sqlite"` | 否 | storage | 存储后端，可选 sqlite、ndjson |
| `storage.retention` | `table` | `Option<StorageRetentionConfig>` | 各字段默认值 | 否 | storage | 为所选后端提供默认保留期 |
| `storage.retention.max_age_seconds` | `integer` | `u32` | `0` | 否 | storage | 共享保留时长，单位秒，0 表示关闭过期清理，后端专属值优先 |
| `storage.backends` | `table` | `StorageBackendConfigs` | 各字段默认值 | 否 | storage | 两种后端的配置容器 |
| `storage.backends.sqlite` | `table` | `Option<SqliteConfig>` | 各字段默认值 | 否 | storage | SQLite 存储设置 |
| `storage.backends.sqlite.path` | `string` | `PathBuf` | `"captures.sqlite"` | 否 | storage | 数据库文件位置，相对路径以所属配置文件目录为基准，须非空 |
| `storage.backends.sqlite.retention` | `table` | `Option<SqliteRetentionConfig>` | 各字段默认值 | 否 | storage | SQLite 记录保留期与批量清理设置 |
| `storage.backends.sqlite.retention.max_age_seconds` | `integer` | `Option<u32>` | 无 | 否 | storage | 后端保留时长，单位秒，未设置时继承 storage.retention.max_age_seconds，0 关闭该后端的过期清理 |
| `storage.backends.sqlite.retention.cleanup_interval_seconds` | `integer` | `Option<NonZeroU32>` | `60` | 否 | storage | 启用保留期时的清理间隔，单位秒，须大于零 |
| `storage.backends.sqlite.retention.delete_batch_size` | `integer` | `Option<NonZeroU32>` | `256` | 否 | storage | 单批删除记录数，须大于零 |
| `storage.backends.ndjson` | `table` | `Option<NdjsonConfig>` | 各字段默认值 | 否 | storage | 按时间和大小轮转的 NDJSON 后端配置 |
| `storage.backends.ndjson.directory` | `string` | `PathBuf` | `"captures"` | 否 | storage | 分段目录，相对路径以所属配置文件目录为基准，须非空 |
| `storage.backends.ndjson.segment_seconds` | `integer` | `NonZeroU32` | `3600` | 否 | storage | 每个时间分段的秒数，须大于零 |
| `storage.backends.ndjson.max_segment_bytes` | `integer` | `NonZeroU64` | `67108864` | 否 | storage | 单个分段文件的字节上限，须大于零 |
| `storage.backends.ndjson.retention` | `table` | `Option<NdjsonRetentionConfig>` | 各字段默认值 | 否 | storage | NDJSON 分段保留期与清理设置 |
| `storage.backends.ndjson.retention.max_age_seconds` | `integer` | `Option<u32>` | 无 | 否 | storage | 后端保留时长，单位秒，未设置时继承 storage.retention.max_age_seconds，0 关闭该后端的过期清理 |
| `storage.backends.ndjson.retention.cleanup_interval_seconds` | `integer` | `Option<NonZeroU32>` | `60` | 否 | storage | 启用保留期时的清理间隔，单位秒，须大于零 |
| `storage.backends.ndjson.retention.max_segments_per_cleanup` | `integer` | `Option<NonZeroU32>` | 无 | 否 | storage | 单次最多清理的分段数，未指定时不设数量上限，指定值须大于零 |
| `analyzer` | `table` | `AnalyzerSettings` | 各字段默认值 | 否 | analyzer | 分析输入、输出、算法参数与筛选设置 |
| `analyzer.input` | `string` | `Option<PathBuf>` | 无 | 否 | analyzer | NDJSON 输入路径，为空时依次使用 receiver.output、共享 storage，--input 可覆盖 |
| `analyzer.output` | `string` | `PathBuf` | `"analysis.json"` | 否 | analyzer | 分析结果 JSON 输出路径，可由 --output 覆盖 |
| `analyzer.options` | `table` | `AnalyzeConfig` | 各字段默认值 | 否 | analyzer | 分析算法阈值和资源上限 |
| `analyzer.options.top_k` | `integer` | `usize` | `20` | 否 | analyzer | 排名输出条数，须大于零，可由 --top-k 覆盖 |
| `analyzer.options.corpus` | `table` | `CorpusConfig` | 各字段默认值 | 否 | analyzer | 语料加载 |
| `analyzer.options.corpus.max_record_bytes` | `integer` | `usize` | `8388608` | 否 | analyzer | 输入记录的字节上限，须大于零 |
| `analyzer.options.corpus.max_records` | `integer` | `usize` | `1000000` | 否 | analyzer | 载入记录数量上限，须大于零 |
| `analyzer.options.projection` | `table` | `ProjectionConfig` | 各字段默认值 | 否 | analyzer | 请求投影 |
| `analyzer.options.projection.comparison_window_seconds` | `integer` | `u64` | `3600` | 否 | analyzer | 比较时间窗的秒数，须大于零，可由 --comparison-window-seconds 覆盖 |
| `analyzer.options.projection.max_units` | `integer` | `usize` | `512` | 否 | analyzer | 每条请求的投影单元数量上限，须大于零 |
| `analyzer.options.projection.max_text_unit_bytes` | `integer` | `usize` | `1048576` | 否 | analyzer | 单个文本投影单元的字节上限，须大于零 |
| `analyzer.options.candidate` | `table` | `CandidateConfig` | 各字段默认值 | 否 | analyzer | 候选构造 |
| `analyzer.options.candidate.min_members` | `integer` | `usize` | `3` | 否 | analyzer | 模板最少成员数，至少为 2 |
| `analyzer.options.candidate.compatibility_threshold` | `float` | `f64` | `0.68` | 否 | analyzer | 模板兼容度阈值，范围 0 到 1 |
| `analyzer.options.candidate.max_dynamic_coverage_ratio` | `float` | `f64` | `0.35` | 否 | analyzer | 动态内容覆盖比例上限，范围 0 到 1 |
| `analyzer.options.candidate.max_candidates_per_request` | `integer` | `usize` | `128` | 否 | analyzer | 每条请求的候选数量上限，须大于零 |
| `analyzer.options.stability` | `table` | `StabilityConfig` | 各字段默认值 | 否 | analyzer | 模板与诊断共享的稳定性条件 |
| `analyzer.options.stability.stable_support_ratio` | `float` | `f64` | `0.80` | 否 | analyzer | 稳定片段支持率，范围 0 到 1 |
| `analyzer.options.stability.min_stable_support` | `integer` | `usize` | `3` | 否 | analyzer | 稳定片段最少支持记录数，至少为 2 |
| `analyzer.options.stability.min_anchor_bytes` | `integer` | `usize` | `24` | 否 | analyzer | 精确匹配锚点最少字节数，须大于零 |
| `analyzer.options.template` | `table` | `TemplateConfig` | 各字段默认值 | 否 | analyzer | 模板提取 |
| `analyzer.options.template.max_alignment_cells` | `integer` | `usize` | `2000000` | 否 | analyzer | 单次对齐单元格上限，须大于零 |
| `analyzer.options.template.max_total_alignment_cells` | `integer` | `usize` | `64000000` | 否 | analyzer | 总对齐单元格上限，须大于零 |
| `analyzer.options.diagnosis` | `table` | `DiagnosisConfig` | 各字段默认值 | 否 | analyzer | 缺陷诊断 |
| `analyzer.options.diagnosis.fixed_variant_max` | `integer` | `usize` | `3` | 否 | analyzer | 固定变体数量上限，须大于零 |
| `analyzer.options.diagnosis.min_blocked_bytes` | `integer` | `usize` | `64` | 否 | analyzer | 纳入阻断分析的稳定内容最少字节数，须大于零 |
| `analyzer.options.prefix_view` | `table` | `PrefixViewConfig` | 各字段默认值 | 否 | analyzer | 前缀树视图 |
| `analyzer.options.prefix_view.max_nodes` | `integer` | `usize` | `5000` | 否 | analyzer | 模板前缀树视图的节点数量上限，0 关闭前缀树视图输出 |
| `analyzer.options.prefix_view.excerpt_bytes` | `integer` | `usize` | `96` | 否 | analyzer | 前缀树节点文本摘录上限，按字符计数，须大于零 |
| `analyzer.selection` | `table` | `SelectionSettings` | 各字段默认值 | 否 | analyzer | 会话、时间区间和最近记录筛选 |
| `analyzer.selection.session_keys` | `array<string>` | `Vec<String>` | `[]` | 否 | analyzer | 允许的会话键集合，空列表匹配全部会话，重复 --session-key 可覆盖 |
| `analyzer.selection.session_keys[]` | `string` | `String` | 无 | 是 | analyzer | 单个会话键，重复值合并 |
| `analyzer.selection.time_ranges` | `array<table>` | `Vec<TimeRangeSettings>` | `[]` | 否 | analyzer | 多个区间取并集，空列表不按时间区间过滤，--time-range 或 --from、--until 可覆盖 |
| `analyzer.selection.time_ranges[]` | `table` | `TimeRangeSettings` | 无 | 是 | analyzer | 单个采集时间区间，双边界须满足 from 小于 until |
| `analyzer.selection.time_ranges[].from` | `string` | `Option<String>` | 无 | 与 until 至少一个非空 | analyzer | 包含的下界，接受 YYYY-MM-DD 或 RFC 3339，日期取 UTC 当日零点 |
| `analyzer.selection.time_ranges[].until` | `string` | `Option<String>` | 无 | 与 from 至少一个非空 | analyzer | 不包含的上界，接受 YYYY-MM-DD 或 RFC 3339，日期取 UTC 当日零点 |
| `analyzer.selection.last_n` | `integer` | `Option<NonZeroUsize>` | 无 | 否 | analyzer | 按采集时间保留最新 N 条匹配记录，未指定时全部，指定值须大于零，--last-n 可覆盖 |
| `export` | `table` | `ExportSettings` | 各字段默认值 | 否 | export | 采集记录导出设置，同时指定时间上下界时须满足 from 小于 until |
| `export.output` | `string` | `PathBuf` | `"exported.ndjson"` | 否 | export | NDJSON 输出位置，可由 --output 覆盖 |
| `export.session_key` | `string` | `Option<String>` | 无 | 否 | export | 精确匹配的单个会话键，未指定时匹配全部，--session-key 可覆盖 |
| `export.from` | `string` | `Option<String>` | 无 | 否 | export | 包含的下界，接受 YYYY-MM-DD 或 RFC 3339，日期取 UTC 当日零点，可由 --from 覆盖 |
| `export.until` | `string` | `Option<String>` | 无 | 否 | export | 不包含的上界，接受 YYYY-MM-DD 或 RFC 3339，日期取 UTC 当日零点，可由 --until 覆盖 |
| `report` | `table` | `ReportSettings` | 各字段默认值 | 否 | report | 报告输入、输出、格式与语言设置 |
| `report.input` | `string` | `Option<PathBuf>` | 无 | 否 | report | 为空时使用 analyzer.output，仍未指定时使用设置目录下 analysis.json，--input 可覆盖 |
| `report.output` | `string` | `Option<PathBuf>` | 无 | 否 | report | 为空时 Markdown 单格式使用 report.md，其余使用 report.html，双格式输出使用同一文件名主体，--output 可覆盖 |
| `report.format` | `string` | `Option<ReportFormat>`（ReportFormat 为 enum） | 无 | 否 | report | 可选 html、md，未指定时同时生成 HTML 和 Markdown，--format 可覆盖 |
| `report.locale` | `string` | `String` | `"zh-CN"` | 否 | report | 支持 zh-CN、en-US，也接受 zh、en、大小写与下划线变体，--locale 可覆盖 |
