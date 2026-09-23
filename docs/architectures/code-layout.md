<!-- 本文件用当前真实源码树说明三个二进制、统一 P1/X/P2 分析核心及模块职责。 -->
# Code Layout

## 三个二进制

```text
采集源
  → actrail-kv-receiver
  → requests.ndjson
  → actrail-kv-analyze
  → analysis.json
  → actrail-kv-report
  → report.html
```

```bash
actrail-kv-receiver --listen 127.0.0.1:8080 --output ./data/requests.ndjson
actrail-kv-analyze --input ./data/requests.ndjson --output ./data/analysis.json --top-k 100
actrail-kv-report --input ./data/analysis.json --output ./data/report.html
```

运行参数见[配置参考](../configuration.md)，三个二进制的输入输出协议见[架构入口](index.md)。

## 当前源码树

```text
actrail-kv/
├── cmd/                                           # 二进制入口；只解析 CLI 并调用对应 library
│   ├── receiver/src/main.rs                       # 启动 HTTP receiver
│   ├── analyze/src/main.rs                        # 执行离线分析；载入 JSON 配置，CLI 覆盖 Top K/时间窗口
│   └── report/src/main.rs                         # 将 analysis.json 渲染为 HTML
│
├── artifacts/src/                                 # 三个二进制共享的磁盘协议
│   ├── captured_request.rs                        # CapturedRequest、显式 ComparisonMetadata
│   ├── analysis_result.rs                         # template、统一 ContextDefect、Top K ID
│   └── lib.rs                                     # 公共 DTO re-export
│
├── receiver/src/                                  # 最小接收端；不依赖 analyzer
│   ├── config.rs                                  # ReceiverConfig：listen/max_payload_bytes JSON 读写
│   ├── http_receiver.rs                           # POST /requests、metadata headers、captured_at
│   ├── ndjson_appender.rs                         # 并发安全、原子行追加、文件独占与权限
│   └── lib.rs                                     # receiver library API
│
├── analyzer/src/                                  # 模板抽取与结构诊断核心
│   ├── cache_metrics.rs                           # 每请求 KV 命中率：上游 usage 优先，缺失时按公共前缀估算
│   ├── config.rs                                  # AnalyzeConfig：20 项阈值/预算 JSON 读写与校验
│   ├── prefix_view.rs                             # 每模板前缀视图模型：稳定/变体片段、变体、缺陷关联
│   ├── model/
│   │   ├── comparison/
│   │   │   ├── group_key.rs                      # 时间窗口+endpoint+model/deployment+schema+agent+namespace
│   │   │   └── mod.rs
│   │   ├── corpus/
│   │   │   ├── loader.rs                         # 有界读取 NDJSON、坏记录 skip、稳定请求 ID
│   │   │   ├── types.rs                          # CorpusRecord、CaptureComparison、skip 类型
│   │   │   └── mod.rs
│   │   ├── projection/
│   │   │   ├── projector.rs                      # 投影 tools/messages/content blocks 的模型可见顺序
│   │   │   ├── projector/tests.rs                # 层级、顺序、预算与非上下文字段测试
│   │   │   ├── types.rs                          # CacheSequence、CacheUnit、HierarchyLocation、source
│   │   │   └── mod.rs
│   │   └── mod.rs
│   │
│   ├── discovery/
│   │   ├── candidate/
│   │   │   ├── signature.rs                      # 有界结构兼容度与候选召回签名
│   │   │   ├── cohort.rs                         # 确定性模板 cohort、bridge 防护和预算
│   │   │   └── mod.rs
│   │   ├── template/
│   │   │   ├── align.rs                          # 有界 unit sequence alignment
│   │   │   ├── extractor.rs                      # stable span、slot、模板坐标与成员映射
│   │   │   ├── extractor/tests.rs                # Unicode、支持率、预算、插入/缺失和置换测试
│   │   │   ├── types.rs                          # RequestTemplate 及抽取结果
│   │   │   ├── symbolic/
│   │   │   │   ├── coordinate.rs                # TemplateCoordinate、逻辑单元 key
│   │   │   │   ├── member_map.rs                # 每成员 coordinate binding、Gap、UnmatchedRun
│   │   │   │   ├── sequence.rs                  # 一等 SymbolicMemberSequence 与 SymbolicAtom
│   │   │   │   ├── build.rs                     # 从 alignment 构造成员坐标与符号序列
│   │   │   │   ├── identity.rs                  # 不依赖成员/绝对 unit index 的坐标 ID
│   │   │   │   ├── tests.rs                     # 成员顺序与公共 prelude 身份不变量
│   │   │   │   └── mod.rs
│   │   │   └── mod.rs
│   │   └── mod.rs
│   │
│   ├── diagnosis/
│   │   ├── episode/
│   │   │   ├── locator.rs                       # 从全部符号成员定位唯一 P1/X/P2 region
│   │   │   ├── variant.rs                       # 每成员一次的 X 变体和 P2 证据构造
│   │   │   ├── model.rs                         # DiagnosisOptions、DefectCandidate、EpisodeVariant
│   │   │   ├── tests.rs                         # P2 门槛、聚合一次性、等价事实和身份稳定
│   │   │   └── mod.rs
│   │   ├── facts/
│   │   │   ├── content.rs                       # 内容变化和少数固定变体事实
│   │   │   ├── representation.rs                # 模型可见 JSON 严格等价事实
│   │   │   ├── sequence.rs                      # 插入/缺失及唯一身份重排事实
│   │   │   └── mod.rs                           # 合并 facts/insights，不拆分同一 episode
│   │   ├── identity/mod.rs                       # 基于逻辑 region 与完整 P2 anchor 的 defect ID
│   │   └── mod.rs
│   │
│   ├── ranking/
│   │   ├── mod.rs                               # 已唯一化 candidate 的评分、稳定全排序、Top K
│   │   └── tests.rs                             # 去重、tie-break 和 Top K 前缀测试
│   ├── session/
│   │   ├── mod.rs                               # 会话 prefix-switch 与结构复用率模块入口
│   │   └── lineage.rs                           # 相邻对 append/fork/reorder/reset、浪费字节、复用率与字节构成
│   ├── run/
│   │   ├── options.rs                           # 算法阈值、时间窗口和资源预算
│   │   ├── pipeline.rs                          # 文件/reader 编排、artifact 映射与原子输出
│   │   └── mod.rs
│   └── lib.rs
│
├── reporter/src/                                # 只消费统一 analysis.json；不重新分析
│   ├── result_loader.rs                         # schema、引用、成员、P2、score、Top K 强校验
│   ├── html_report_renderer.rs                  # comparison group、P1/X/P2、facts、insights；全转义
│   └── lib.rs
│
├── examples/
│   ├── quickstart/
│   │   └── requests.ndjson                      # 根 README 可直接分析的最小缺陷语料
│   ├── cases/                                  # 真实语料回归 fixture（workspace/policy/json/insert/natural/session）
│   ├── analyze.config.example.json              # analyze 全字段 JSON 配置示例
│   └── receiver.config.example.json             # receiver JSON 配置示例
│
└── tests/end_to_end/
    └── three_binaries.sh                        # 真实 receiver→analyze→report、确定性与 XSS 验收
```

## 核心调用链

```text
analyze_reader
  ├─ CorpusLoader.load
  ├─ RequestProjector.project
  │    └─ ComparisonGroupKey.from_record
  ├─ CandidateBuilder.build
  ├─ TemplateExtractor.extract
  │    ├─ SequenceAligner.align
  │    └─ symbolic::build member maps + sequences
  ├─ diagnose_template
  │    ├─ episode::locator locate P1/X/P2
  │    ├─ episode::variant aggregate one variant per member
  │    ├─ facts analyze content/sequence/representation
  │    └─ identity build stable defect ID
  ├─ rank_defects
  └─ write_atomic_json
```

`ranking` 不再从请求对猜根因；它只消费已经唯一化的 `DefectCandidate`。scope、内容版本和 JSON 等价都是同一个 episode 的 facts，不会各自生成 defect。
