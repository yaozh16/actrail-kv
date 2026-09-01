<!-- 本文件用一棵完整目录树说明三个二进制及基于可观察请求结构的离线诊断核心。 -->
# Code Layout

## 运行方式

```bash
# 常驻接收采集方提交的完整模型 HTTP request payload
actrail-kv-receiver --listen 0.0.0.0:8080 --output ./data/requests.ndjson

# 离线抽模板、诊断结构性缓存破坏并计算 Top K
actrail-kv-analyze --input ./data/requests.ndjson --output ./data/analysis.json --top-k 100

# 生成报告
actrail-kv-report --input ./data/analysis.json --output ./data/report.html
```

```text
请求采集源（网关 / 插桩组件 / Agent 框架 Hook / 其他采集器）
  → actrail-kv-receiver
  → requests.ndjson
  → actrail-kv-analyze
  → analysis.json
  → actrail-kv-report
  → report.html
```

## 分析边界

MVP 不限定请求由谁采集，只假设采集方能够提交发往模型 API 的完整 HTTP JSON payload，其中包含 `model`、`messages`、`tools` 及其他请求字段。系统没有模型内部 tokenizer、chat template 或真实 KV 命中信息。因此：

- 不生成或比较 Token ID。
- 不声称计算了模型内部的精确 KV 前缀。
- 比较完整请求中可观察的有序字段、原始 UTF-8 文本和结构片段。
- 使用“被阻断的稳定字节/字符/片段”作为结构机会代理指标。
- 所有结论都必须标记为结构优化启示，而不是真实缓存收益。

## 完整目录

下列内容是一棵完整代码树。文件名暂不绑定具体语言扩展名。

```text
actrail-kv/
│
├── cmd/                                         # 三个二进制统一放在一起；入口只解析参数、组装对象和返回退出码
│   ├── receiver/
│   │   └── main                                 # 构建并启动 actrail-kv-receiver
│   ├── analyze/
│   │   └── main                                 # 构建并运行 actrail-kv-analyze
│   └── report/
│       └── main                                 # 构建并运行 actrail-kv-report
│
├── artifacts/                                   # 三个二进制之间唯一共享的磁盘格式
│   ├── captured_request                         # requests.ndjson 单行：captured_at、可选 source、原始 HTTP JSON payload
│   └── analysis_result                          # analysis.json：运行信息、模板、Findings 和 Top K
│
├── receiver/                                    # actrail-kv-receiver 的实现；不关心上游是网关还是插桩组件
│   ├── http_receiver                            # POST /requests；读取采集方提交的完整模型 HTTP JSON payload
│   └── ndjson_appender                          # 补 captured_at，原样追加 payload，不解释请求内容
│
├── analyzer/                                    # actrail-kv-analyze 的实现；项目核心代码全部在这里
│   │
│   ├── run/                                     # 单次离线分析的入口和阶段编排
│   │   ├── analysis_options                     # top_k、阈值、字段选择、允许的结构变换
│   │   ├── analysis_context                     # 本次运行共享的只读配置和统计量
│   │   ├── analysis_pipeline                    # run(records, options) -> AnalysisResult
│   │   └── analysis_run                         # 输入范围、样本数、阶段耗时、跳过原因和参数快照
│   │
│   ├── model/                                   # 把请求原文变成可比较且能回溯来源的分析模型
│   │   │
│   │   ├── corpus/
│   │   │   ├── corpus_loader                    # 流式读取 CapturedRequest，建立 AnalysisCorpus
│   │   │   ├── payload_parser                   # 直接解析 payload 中的 model、messages、tools 等字段
│   │   │   ├── request_payload                  # 保留完整 payload 及解析后的字段访问入口
│   │   │   ├── request_part                     # payload 内的 role、content、tool definition、tool result 等片段
│   │   │   ├── source_location                  # 原 JSON path、消息索引、角色和字段名
│   │   │   └── analysis_corpus                  # payload 集合及按模型、时间和结构建立的只读索引
│   │   │
│   │   └── projection/
│   │       ├── field_policy                     # 声明 payload 中参与结构分析的字段及其逻辑顺序
│   │       ├── request_projector                # 按 payload 的 messages/tools 等有序结构投影为 CacheSequence
│   │       ├── cache_unit                       # 字段边界、结构标记或原始 UTF-8 内容片段
│   │       ├── cache_sequence                   # 有序 CacheUnit 列表；不是模型 Token 序列
│   │       └── unit_origin_map                  # 每个 CacheUnit/字节区间回到 SourceLocation
│   │
│   ├── discovery/                               # 从大量请求中找到模板及其稳定/动态区域
│   │   │
│   │   ├── candidate/
│   │   │   ├── structure_signature              # 用字段形状、角色序列、工具集合等生成粗签名
│   │   │   ├── candidate_index                  # 按模型和结构签名索引可能共享模板的请求
│   │   │   ├── cohort_builder                   # 生成待抽取模板的 CandidateCohort
│   │   │   ├── candidate_cohort                 # 可能属于同一模板的一组请求引用
│   │   │   └── candidate_stats                  # 组大小、过滤数量和过滤原因
│   │   │
│   │   └── template/
│   │       ├── sequence_aligner                 # 对齐 cohort 内的 CacheUnit 和文本区间
│   │       ├── stability_estimator              # 统计片段或字节区间的重复率
│   │       ├── slot_inferer                     # 将变化区间抽取成带来源字段的动态槽位
│   │       ├── stable_span                      # 稳定内容、位置、长度和支持请求数
│   │       ├── template_slot                    # 动态内容、SourceLocation、取值统计和置信度
│   │       ├── request_template                 # StableSpan[] + TemplateSlot[] + members
│   │       └── template_extractor               # 协调对齐、稳定性估计和槽位抽取
│   │
│   ├── diagnosis/                               # 比较实际结构与安全反事实，解释缓存机会为何被破坏
│   │   │
│   │   ├── prefix/
│   │   │   ├── byte_prefix_trie                 # 计算 cohort 的公共 UTF-8 字节前缀，避免请求对枚举
│   │   │   ├── structural_prefix                # 公共 CacheUnit 前缀及首个不同单元
│   │   │   ├── actual_prefix                    # 当前请求组织下的公共字节/字符/片段长度
│   │   │   ├── transform_policy                 # 允许的规范化、稳定排序或字段后移规则
│   │   │   ├── counterfactual_builder           # 应用允许的变换并记录每一步影响的字段
│   │   │   ├── potential_prefix                 # 安全反事实下的潜在公共前缀
│   │   │   └── prefix_opportunity               # actual、potential、blocked、affected requests
│   │   │
│   │   └── attribution/
│   │       ├── divergence_locator               # 定位公共前缀终止处的 CacheUnit/字节区间
│   │       ├── source_attributor                # 经 UnitOriginMap 回溯到原请求 JSON path
│   │       ├── cause_classifier                 # 分类动态前置、顺序漂移、序列化噪声等原因
│   │       ├── evidence_builder                 # 生成代表请求差异和被阻断稳定区域
│   │       ├── recommendation_builder           # 根据原因及允许的变换生成优化建议
│   │       └── diagnostic_case                  # 位置、原因、证据、反事实、影响范围和置信度
│   │
│   └── ranking/                                 # 将大量 case 聚合成可行动的问题模式并选 Top K
│       ├── problem_fingerprint                  # template + cause + source path + fix action
│       ├── finding_aggregator                   # 按 fingerprint 聚合，避免请求对刷榜
│       ├── opportunity_scorer                   # blocked stable bytes × affected count × confidence
│       ├── score_breakdown                      # 保存各评分因子，保证排序可以解释
│       ├── finding                              # 聚合问题、影响范围、代表证据和建议
│       ├── top_k_selector                       # 稳定排序、并列处理和 Top K 截断
│       └── result_builder                       # 生成最终 AnalysisResult
│
├── reporter/                                    # actrail-kv-report 的实现；非核心，保持最小
│   ├── result_loader                            # 读取 AnalysisResult
│   └── html_report_renderer                     # 展示 Top K、分项得分、证据、限制和建议
│
└── tests/
    ├── fixtures/                                # 请求语料及期望模板、Finding 和 Top K
    ├── analyzer/                                # 与 analyzer 各算法目录对应的测试
    │   ├── model/
    │   ├── discovery/
    │   ├── diagnosis/
    │   └── ranking/
    └── end_to_end/
        └── three_binaries                       # receiver → analyzer → reporter 文件级串联
```

## 核心调用顺序

```text
AnalysisPipeline.run
  ├── CorpusLoader.load
  ├── RequestProjector.project
  ├── CandidateIndex.add + CohortBuilder.build
  ├── TemplateExtractor.extract
  ├── BytePrefixTrie.common_prefix + StructuralPrefix.compare
  ├── CounterfactualBuilder.build + PotentialPrefix.calculate
  ├── DivergenceLocator.locate + SourceAttributor.attribute
  ├── CauseClassifier.classify + EvidenceBuilder.build
  ├── FindingAggregator.aggregate + OpportunityScorer.score
  └── TopKSelector.select + ResultBuilder.build
```

`receiver` 和 `reporter` 不依赖 `analyzer`。核心实现及测试集中在 `analyzer/`；缓存影响统一以可观察请求结构的代理指标表达。
