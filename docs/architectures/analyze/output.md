<!-- 本文件定义 actrail-kv-analyze 产物 analysis.json 的统一 P1/X/P2 字段协议和排序契约。 -->
# Analyze 输出

`actrail-kv-analyze` 输出一个 JSON object。字段路径中的 `[]` 表示数组成员，`?` 表示字段可以省略。

## 字段协议

| 字段路径 | 类型 | 含义 |
|---|---|---|
| `run` | object | 本次分析运行摘要。 |
| `run.schema_version` | string | 当前为 `0.1.0`。 |
| `run.input_records` | integer | 读取的 NDJSON 记录数。 |
| `run.analyzed_records` | integer | 成功投影并进入分析的记录数。 |
| `run.skipped_records[]` | object array | 未进入分析的记录。 |
| `run.skipped_records[].record_index` | integer | 输入文件中的一基行号。 |
| `run.skipped_records[].reason` | string | 跳过原因。 |
| `run.options` | object | 本次实际生效的参数快照。 |
| `run.options.top_k` | integer | Top K 数量。 |
| `run.options.comparison_window_seconds` | integer | 固定 UTC comparison window 宽度。 |
| `run.options.min_template_members` | integer | 模板最小成员数。 |
| `run.options.stable_span_support_ratio` | number | 稳定片段最小支持率。 |
| `run.options.min_stable_support` | integer | 稳定片段最小支持数。 |
| `run.options.fixed_variant_max` | integer | 少数固定版本判定阈值。 |
| `run.options.min_blocked_stable_bytes` | integer | 形成 defect 所需的最小 P2 bytes。 |
| `run.options.min_exact_anchor_bytes` | integer | P2 精确锚点最小 bytes。 |
| `run.options.text_similarity_threshold` | number | 候选文本相似度门槛。 |
| `run.options.template_compatibility_threshold` | number | 模板结构兼容门槛。 |
| `run.options.max_dynamic_coverage_ratio` | number | 模板最大动态覆盖率。 |
| `run.options.max_candidates_per_request` | integer | 单请求候选召回上限。 |
| `run.options.max_projection_units` | integer | 单请求投影单元上限。 |
| `run.options.max_text_unit_bytes` | integer | 单文本单元 byte 上限。 |
| `run.options.max_payload_bytes` | integer | 单输入记录 byte 上限。 |
| `run.options.max_alignment_cells` | integer | 单次对齐 cell 上限。 |
| `run.options.max_total_alignment_cells` | integer | 单 cohort 累计对齐 cell 上限。 |
| `run.options.max_records` | integer | 单次加载记录数上限。 |
| `run.limitations?[]` | string array | 本次结果的解释限制。 |
| `templates[]` | object array | comparison group 内抽取的请求模板。 |
| `templates[].id` | string | 模板稳定 ID。 |
| `templates[].comparison_group` | object | 该模板的比较边界。 |
| `templates[].comparison_group.time_window_key` | string | 固定 UTC 时间窗口起点的 Unix seconds。 |
| `templates[].comparison_group.endpoint_key` | string | endpoint 或逻辑路由。 |
| `templates[].comparison_group.model` | string | payload 中的模型。 |
| `templates[].comparison_group.context_schema_key` | string | adapter 生成的模型可见结构版本。 |
| `templates[].comparison_group.agent_key?` | string | 可选 Agent key。 |
| `templates[].comparison_group.model_deployment_key?` | string | 可选模型部署 key。 |
| `templates[].comparison_group.kv_namespace?` | string | 可选真实 KV namespace。 |
| `templates[].member_request_ids[]` | string array | 模板成员的稳定请求 ID。 |
| `templates[].medoid_request_id` | string | 确定性对齐基准成员。 |
| `templates[].cohesion` | number，`0..1` | 模板结构一致性。 |
| `templates[].stable_spans[]` | object array | 达到支持门槛的稳定文本片段。 |
| `templates[].stable_spans[].source` | object | 稳定片段来源。 |
| `templates[].stable_spans[].source.json_path` | string | 原 payload JSONPath。 |
| `templates[].stable_spans[].source.logical_scope[]` | string array | 模型可见层级定位。 |
| `templates[].stable_spans[].source.unit_index?` | integer | 投影单元序号。 |
| `templates[].stable_spans[].source.byte_start?` | integer | UTF-8 起始 byte offset。 |
| `templates[].stable_spans[].source.byte_end?` | integer | UTF-8 排他结束 byte offset。 |
| `templates[].stable_spans[].source.role?` | string | message role。 |
| `templates[].stable_spans[].utf8_bytes` | integer | 稳定片段 bytes。 |
| `templates[].stable_spans[].support_count` | integer | 支持成员数。 |
| `templates[].stable_spans[].excerpt?` | string | 有界证据摘录。 |
| `templates[].slots[]` | object array | 模板动态槽位摘要。 |
| `templates[].slots[].source` | object | 槽位来源。 |
| `templates[].slots[].source.json_path` | string | 原 payload JSONPath。 |
| `templates[].slots[].source.logical_scope[]` | string array | 模型可见层级定位。 |
| `templates[].slots[].source.unit_index?` | integer | 投影单元序号。 |
| `templates[].slots[].source.byte_start?` | integer | UTF-8 起始 byte offset。 |
| `templates[].slots[].source.byte_end?` | integer | UTF-8 排他结束 byte offset。 |
| `templates[].slots[].source.role?` | string | message role。 |
| `templates[].slots[].member_count` | integer | 参与该槽位的成员数。 |
| `templates[].slots[].distinct_variant_count` | integer | 不同槽位值数量。 |
| `templates[].slots[].confidence` | number，`0..1` | 槽位识别置信度。 |
| `defects[]` | object array | 全部唯一化的 `P1 / X / P2` 结构问题。 |
| `defects[].id` | string | 结构稳定 ID，由 `top_k[]` 引用。 |
| `defects[].comparison_group` | object | 问题的比较边界。 |
| `defects[].comparison_group.time_window_key` | string | 固定 UTC 时间窗口起点的 Unix seconds。 |
| `defects[].comparison_group.endpoint_key` | string | endpoint 或逻辑路由。 |
| `defects[].comparison_group.model` | string | payload 中的模型。 |
| `defects[].comparison_group.context_schema_key` | string | adapter 生成的模型可见结构版本。 |
| `defects[].comparison_group.agent_key?` | string | 可选 Agent key。 |
| `defects[].comparison_group.model_deployment_key?` | string | 可选模型部署 key。 |
| `defects[].comparison_group.kv_namespace?` | string | 可选真实 KV namespace。 |
| `defects[].template_id` | string | 关联的 `templates[].id`。 |
| `defects[].mismatch` | object | 失配区域 X。 |
| `defects[].mismatch.pattern` | string enum | `value_mismatch`、`insertion_deletion`、`reorder` 或 `mixed`。 |
| `defects[].mismatch.variants[]` | object array | X 的符号变体分组，至少两个。 |
| `defects[].mismatch.variants[].fingerprint` | string | 不含原始动态值的变体摘要。 |
| `defects[].mismatch.variants[].member_request_ids[]` | string array | 属于该变体的请求 ID。不同变体不得重叠。 |
| `defects[].mismatch.variants[].representative` | object | 该变体的有界代表证据。 |
| `defects[].mismatch.variants[].representative.request_id` | string | 代表请求，必须属于本变体。 |
| `defects[].mismatch.variants[].representative.sources[]` | object array | X 在代表请求中的一个或多个来源；缺失变体可以为空。 |
| `defects[].mismatch.variants[].representative.sources[].json_path` | string | 原 payload JSONPath。 |
| `defects[].mismatch.variants[].representative.sources[].logical_scope[]` | string array | X 所在的 system/tool/history/content 层级。 |
| `defects[].mismatch.variants[].representative.sources[].unit_index?` | integer | 投影单元序号。 |
| `defects[].mismatch.variants[].representative.sources[].byte_start?` | integer | UTF-8 起始 byte offset。 |
| `defects[].mismatch.variants[].representative.sources[].byte_end?` | integer | UTF-8 排他结束 byte offset。 |
| `defects[].mismatch.variants[].representative.sources[].role?` | string | message role。 |
| `defects[].mismatch.variants[].representative.utf8_bytes` | integer | 代表 X 变体的可观察 bytes。 |
| `defects[].mismatch.variants[].representative.excerpt?` | string | 有界 X 摘录。 |
| `defects[].mismatch.facts[]` | object array | 对同一个 X 的结构事实，不会拆成额外 defect。 |
| `defects[].mismatch.facts[].kind` | string enum | `content_variation`、`fixed_variants`、`structured_data_equivalent`、`insertion_deletion`、`reorder` 或 `scope`。 |
| `defects[].mismatch.facts[].detail?` | string | 事实说明或 scope 定位。 |
| `defects[].recovered_stable` | object | 分歧后重新出现的稳定 P2。 |
| `defects[].recovered_stable.sources[]` | object array | P2 的一个或多个来源。 |
| `defects[].recovered_stable.sources[].json_path` | string | 原 payload JSONPath。 |
| `defects[].recovered_stable.sources[].logical_scope[]` | string array | P2 所在的模型可见层级。 |
| `defects[].recovered_stable.sources[].unit_index?` | integer | 投影单元序号。 |
| `defects[].recovered_stable.sources[].byte_start?` | integer | UTF-8 起始 byte offset。 |
| `defects[].recovered_stable.sources[].byte_end?` | integer | UTF-8 排他结束 byte offset。 |
| `defects[].recovered_stable.sources[].role?` | string | message role。 |
| `defects[].recovered_stable.utf8_bytes` | integer | 保守的连续 P2 bytes。 |
| `defects[].recovered_stable.support_count` | integer | 支持该 P2 并参与本问题的成员数。 |
| `defects[].recovered_stable.excerpt` | string | 有界 P2 摘录。 |
| `defects[].actual_prefix_bytes` | integer | 当前 P1 的可观察 bytes。 |
| `defects[].potential_prefix_bytes` | integer | 结构反事实下 `P1 + P2` 的 bytes。 |
| `defects[].blocked_stable_bytes` | integer | 被 X 阻断的保守 P2 bytes。 |
| `defects[].comparable_count` | integer | 支持同一 P2 并进入 X 变体统计的成员数。 |
| `defects[].affected_count` | integer | `comparable_count - 最大变体成员数`。 |
| `defects[].confidence` | number，`0..1` | 模板、P2 支持与结构事实的组合置信度。 |
| `defects[].score` | object | 可解释评分。 |
| `defects[].score.blocked_stable_bytes` | integer | 评分使用的 P2 bytes。 |
| `defects[].score.affected_count` | integer | 评分使用的受影响请求数。 |
| `defects[].score.confidence` | number | 评分使用的置信度。 |
| `defects[].score.score` | number | `blocked_stable_bytes × affected_count × confidence`。 |
| `defects[].insights[]` | object array | 根据结构事实生成的中文优化启示，至少一项。 |
| `defects[].insights[].summary` | string | 优化方向摘要。 |
| `defects[].insights[].detail?` | string | 适用前提或补充说明。 |
| `top_k[]` | string array | 按优先级排列的 defect ID，是完整稳定排序的前缀。 |
| `session_reports[]?` | object array | 会话内 prefix-switch 证据；旧版文件缺省为空。 |
| `session_reports[].session_key` | string | 会话标识。 |
| `session_reports[].endpoint_key` / `.model` | string | 该会话所在比较域的 endpoint 与 model。 |
| `session_reports[].request_count` | integer | 会话内带投影记录数。 |
| `session_reports[].append_count` / `.fork_count` / `.reorder_count` / `.reset_count` | integer | 相邻对四类事件计数。 |
| `session_reports[].total_recomputed_bytes` | integer | 各事件 `next_total - lcp` 之和。 |
| `session_reports[].total_stable_after_switch_bytes` | integer | 切换后连续可恢复稳定字节之和（CC 证据）。 |
| `session_reports[].total_prefix_cut_bytes` / `.avg_prefix_cut_bytes` | integer | 前缀收缩总量与按事件平均（首事件视为 0）。 |
| `session_reports[].events[]` | object array | 相邻对事件明细。 |
| `session_reports[].events[].event_type` | enum | `append` / `fork` / `reorder` / `reset`；仅证据分类，不做业务判断。 |
| `session_reports[].events[].prev_request_id` / `.next_request_id` | string | 参与比较的相邻请求 ID。 |
| `session_reports[].events[].lcp_units` / `.lcp_bytes` | integer | 公共前缀长度。 |
| `session_reports[].events[].previous_pair_lcp_bytes` / `.prefix_cut_bytes` | integer | 前一对公共前缀与本次收缩量（收缩为 0 时不放大）。 |
| `session_reports[].events[].next_total_bytes` / `.recomputed_bytes` | integer | 下一条总字节与切换后需重算字节。 |
| `session_reports[].events[].stable_after_switch_bytes` | integer | 分歧点后连续对齐稳定段字节；可为 0。 |
| `session_reports[].events[].next_block_runs` | string | 下一条内容性单位稳定块 run-length 摘要（如 `aabbccddx4/…`）。 |
| `cache_metrics[]?` | object array | 每请求 KV 命中率指标；旧版文件缺省为空。 |
| `cache_metrics[].request_id` | string | 请求 ID。 |
| `cache_metrics[].session_key?` | string | 会话标识；未采集会话时为缺省。 |
| `cache_metrics[].sequence` | integer | 会话内序号（无 session 时按输入顺序）。 |
| `cache_metrics[].captured_at?` | string | 采集时间（RFC 3339，原样透传）。 |
| `cache_metrics[].source?` | string | 采集来源（原样透传）。 |
| `cache_metrics[].prompt_tokens?` | integer | 上游返回的输入 token 数；估算口径缺省。 |
| `cache_metrics[].cached_tokens?` | integer | 上游返回的命中缓存 token 数（OpenAI `cached_tokens` / Anthropic `cache_read_input_tokens`）；估算口径缺省。 |
| `cache_metrics[].miss_tokens?` | integer | `prompt_tokens - cached_tokens`；估算口径缺省。 |
| `cache_metrics[].output_tokens?` | integer | 上游返回的输出 token 数；估算口径缺省。 |
| `cache_metrics[].hit_rate` | number，`0..1` | `reported` 时为 `cached_tokens / prompt_tokens`；`estimated` 时为 `estimated_lcp_bytes / payload_bytes`（字节比，不是 token 比）。 |
| `cache_metrics[].hit_rate_delta?` | number | 相对同一会话上一条请求的命中率变化；首条缺省。 |
| `cache_metrics[].basis` | enum | `reported`（来自上游 usage）或 `estimated`（按与上一条请求 payload 的公共前缀估算）。 |
| `cache_metrics[].estimated_lcp_bytes?` | integer | 估算口径下与上一条请求 payload 的公共前缀字节数；`reported` 时缺省。 |
| `cache_metrics[].payload_bytes` | integer | 该请求 payload 的 UTF-8 字节数（估算时已剔除保留字段）。 |
| `cache_metrics[].ttft_ms?` | integer | 首 token 时延（毫秒），来自 payload 保留字段；未上报时缺省（`0` 表示真实测得 0）。 |
| `cache_metrics[].total_ms?` | integer | 总耗时（毫秒，wall time：请求发出到流结束）；未上报时缺省。 |

所有 byte range 必须同时提供 start/end、满足 `start <= end`，并落在合法 UTF-8 字符边界。一个 defect 的所有 variant 成员总数等于 `comparable_count`，`recovered_stable.support_count` 也等于该值。

## 示例

```json
{
  "run": {
    "schema_version": "0.1.0",
    "input_records": 4,
    "analyzed_records": 4,
    "skipped_records": [],
    "options": {
      "top_k": 20,
      "comparison_window_seconds": 3600,
      "min_template_members": 3,
      "stable_span_support_ratio": 0.8,
      "min_stable_support": 3,
      "min_blocked_stable_bytes": 64,
      "min_exact_anchor_bytes": 24,
      "text_similarity_threshold": 0.8,
      "template_compatibility_threshold": 0.68,
      "max_dynamic_coverage_ratio": 0.35,
      "max_candidates_per_request": 128,
      "max_projection_units": 512,
      "max_text_unit_bytes": 1048576,
      "max_payload_bytes": 8388608,
      "max_alignment_cells": 2000000,
      "max_total_alignment_cells": 64000000,
      "max_records": 1000000
    },
    "limitations": ["结构代理：未使用 tokenizer、模型内部 chat template 或真实 KV 命中数据"]
  },
  "templates": [{
    "id": "template-a",
    "comparison_group": {
      "time_window_key": "1788336000",
      "endpoint_key": "llm-primary",
      "model": "model-a",
      "context_schema_key": "openai-compatible-chat/v1",
      "agent_key": "coding-agent"
    },
    "member_request_ids": ["req-a", "req-b", "req-c", "req-d"],
    "medoid_request_id": "req-a",
    "cohesion": 0.91,
    "stable_spans": [],
    "slots": []
  }],
  "defects": [{
    "id": "defect-a",
    "comparison_group": {
      "time_window_key": "1788336000",
      "endpoint_key": "llm-primary",
      "model": "model-a",
      "context_schema_key": "openai-compatible-chat/v1",
      "agent_key": "coding-agent"
    },
    "template_id": "template-a",
    "mismatch": {
      "pattern": "value_mismatch",
      "variants": [
        {"fingerprint":"variant-a","member_request_ids":["req-a","req-b"],"representative":{"request_id":"req-a","sources":[{"json_path":"$.messages[0].content","logical_scope":["messages","system"],"unit_index":1,"byte_start":0,"byte_end":13,"role":"system"}],"utf8_bytes":13,"excerpt":"tenant: north"}},
        {"fingerprint":"variant-b","member_request_ids":["req-c","req-d"],"representative":{"request_id":"req-c","sources":[{"json_path":"$.messages[0].content","logical_scope":["messages","system"],"unit_index":1,"byte_start":0,"byte_end":13,"role":"system"}],"utf8_bytes":13,"excerpt":"tenant: south"}}
      ],
      "facts": [
        {"kind":"fixed_variants","detail":"观察到 2 个内容变体"},
        {"kind":"scope","detail":"messages/system"}
      ]
    },
    "recovered_stable": {
      "sources": [{"json_path":"$.messages[1].content","logical_scope":["messages","system"],"unit_index":3,"byte_start":0,"byte_end":128,"role":"system"}],
      "utf8_bytes": 128,
      "support_count": 4,
      "excerpt": "Keep the following policy unchanged..."
    },
    "actual_prefix_bytes": 0,
    "potential_prefix_bytes": 128,
    "blocked_stable_bytes": 128,
    "comparable_count": 4,
    "affected_count": 2,
    "confidence": 0.91,
    "score": {"blocked_stable_bytes":128,"affected_count":2,"confidence":0.91,"score":232.96},
    "insights": [{"summary":"统一稳定内容的配置或模板版本","detail":"同一逻辑位置的变化过早阻断了后续稳定前缀"}]
  }],
  "top_k": ["defect-a"]
}
```

## 排序契约

- score、affected count、blocked stable bytes 依次降序，最后按 defect ID 升序打破并列。
- `top_k=n` 是完整排序的前 n 个 ID，不复制 score 或展示文案。
- 输入顺序、成员顺序和展示文案变化不改变问题身份或排序。
- UTF-8 bytes 是结构代理，不代表真实 token、KV miss、延迟或金额收益。
- `cache_metrics[]` 按会话分组、组内按 `captured_at` 与输入行序稳定排序；缺少 session 的记录全部归入同一个缺省分组。
- `basis=reported` 表示计数来自上游响应的 usage；`basis=estimated` 表示上游没有返回 usage，
  此时 token 字段全部缺省，`hit_rate` 是"与上一条请求 payload 的公共前缀字节 / 本请求 payload 字节"，
  属于字节比代理值。**下游不得把 `estimated` 的 `hit_rate` 当作真实 KV 命中率使用**，
  两者的语义与可信度不同，报告中必须分别标注。
- 保留字段缺少总输入数（`prompt_tokens` / `input_tokens` 都缺）时按没有 usage 处理，
  该请求走 `estimated` 口径；不会用缺失值凑出 0% 的"真实"命中率。
- `cache_metrics[]` 等字段受严格 schema 校验（未知字段直接报错）：analyzer 与 report 需同版本配套，
  用旧版 report 二进制读取新版 `analysis.json` 会失败，而不是静默忽略新字段。
