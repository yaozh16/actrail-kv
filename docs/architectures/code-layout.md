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
│   ├── analysis_result.rs                         # template、direct defect、conditional local site、Top K ID
│   ├── session_analysis.rs                        # Session timeline、transition、prefix metrics、history site
│   └── lib.rs                                     # 公共 DTO re-export
│
├── receiver/src/                                  # 最小接收端；不依赖 analyzer
│   ├── config.rs                                  # ReceiverConfig：listen/max_payload_bytes JSON 读写
│   ├── http_receiver.rs                           # POST /requests、metadata headers、captured_at
│   ├── ndjson_appender.rs                         # 并发安全、原子行追加、文件独占与权限
│   └── lib.rs                                     # receiver library API
│
├── analyzer/src/                                  # 模板抽取与结构诊断核心
│   ├── config.rs                                  # AnalyzeConfig：阈值、预算及兼容字段 JSON 读写与校验
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
│   │   │   ├── signature.rs                      # shift-robust bottom-k shingle 候选召回签名
│   │   │   ├── cohort.rs                         # 确定性模板 cohort、bridge 防护和预算
│   │   │   └── mod.rs
│   │   ├── template/
│   │   │   ├── align.rs                          # 有界 unit sequence alignment
│   │   │   ├── extractor.rs                      # stable span、slot、文本 LCS、模板坐标与成员映射
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
│   │   │   ├── locator.rs                       # 编排同模板 direct 与 conditional episodes
│   │   │   ├── scanner.rs                       # 有界扫描最多四个有序、非重叠 P1/X/P2 region
│   │   │   ├── variant.rs                       # 每成员一次的 X 变体和 P2 证据构造
│   │   │   ├── model.rs                         # DiagnosisOptions、DefectCandidate、EpisodeVariant
│   │   │   ├── tests/
│   │   │   │   ├── mod.rs                     # P2 门槛、聚合一次性、等价事实和身份稳定
│   │   │   │   └── review.rs                  # 恢复区边界、条件身份与 episode 预算回归
│   │   │   └── mod.rs
│   │   ├── session/
│   │   │   ├── model.rs                         # Session observation 与开放投影视图
│   │   │   ├── prefix.rs                        # append-aware 最长公共结构前缀与分类
│   │   │   ├── timeline.rs                      # 时间排序、相邻 transition 与边界
│   │   │   ├── aggregate.rs                     # history transition 按逻辑位置聚合
│   │   │   ├── tests.rs                         # 追加、改写、截断、边界、Unicode 与顺序测试
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
│   ├── run/
│   │   ├── options.rs                           # 算法阈值、时间窗口和资源预算
│   │   ├── pipeline.rs                          # 文件/reader 编排、artifact 映射与原子输出
│   │   └── mod.rs
│   └── lib.rs
│
├── analyzer/tests/
│   ├── acceptance.rs                            # 结构诊断、变体与多位点验收
│   ├── case_fixtures.rs                         # examples/cases 文件入口回归
│   ├── files.rs                                 # 输入输出文件保护与空语料拒绝
│   ├── properties.rs                            # 业务值、稳定尾部及动态后缀性质
│   ├── scale.rs                                 # 一万条同质请求回归
│   └── session_timeline.rs                      # 追加后历史改写的文件入口验收
│
├── reporter/src/                                # 只消费统一 analysis.json；不重新分析
│   ├── result_loader/
│   │   ├── mod.rs                               # schema、模板、Top K 与分层结果编排校验
│   │   ├── common.rs                            # comparison group、source、概率公共校验
│   │   ├── defect.rs                            # direct defect 与 conditional site 强校验
│   │   ├── options.rs                           # Analyzer 运行参数快照边界校验
│   │   ├── session.rs                           # timeline、transition metrics 与 history site 强校验
│   │   └── tests.rs                             # 合法 fixture 与引用/计量拒绝路径
│   ├── html_report_renderer.rs                  # Top K、direct defect 与 conditional site；全转义
│   ├── session_report_renderer.rs               # Session 摘要、时间线与 history sites；全转义
│   └── lib.rs
│
├── examples/
│   ├── quickstart/
│   │   └── requests.ndjson                      # 根 README 可直接分析的最小缺陷语料
│   ├── cases/                                   # 真实 Agent 请求回归语料
│   ├── analyze.config.example.json              # analyze 全字段 JSON 配置示例
│   └── receiver.config.example.json             # receiver JSON 配置示例
│
├── docs/deployment/
│   ├── fullchain-deploy-linux.sh                # Linux 环境与路径解析、录制链管理入口
│   ├── fullchain-deploy-linux/
│   │   └── report.sh                            # analyze/report 调用与 Session 统计
│   └── fullchain-deploy-windows.ps1             # Windows 录制链管理入口
│
└── tests/end_to_end/
    └── three_binaries.sh                        # 三程序、确定性、XSS、裸输出路径与兼容配置验收
```

## 核心调用链

```text
analyze_reader
  ├─ CorpusLoader.load
  ├─ RequestProjector.project
  │    └─ ComparisonGroupKey.from_record
  ├─ session::analyze_session
  │    ├─ timeline sort and adjacent pairing
  │    ├─ prefix classify one frontier per transition
  │    └─ aggregate history sites
  ├─ CandidateBuilder.build
  ├─ TemplateExtractor.extract
  │    ├─ SequenceAligner.align
  │    └─ symbolic::build member maps + sequences
  ├─ diagnose_template
  │    ├─ episode::scanner locate bounded P1/X/P2 chain
  │    ├─ episode::variant aggregate one variant per member
  │    ├─ facts analyze content/sequence/representation
  │    └─ identity build stable episode IDs
  ├─ rank_defects
  └─ write_atomic_json
```

`ranking` 不再从请求对猜根因；它只消费已经唯一化的 `DefectCandidate`。第一个 episode 映射为 direct defect 并参与 Top K，后续 episode 映射为 conditional local sites 并分层输出。Session timeline 独立建模；三类结果不合并字节或分数。scope、内容版本和 JSON 等价都是同一个 episode 的 facts，不会各自生成 defect。
