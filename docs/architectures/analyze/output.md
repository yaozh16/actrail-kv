<!-- 本文件定义 actrail-kv-analyze 产物 analysis.json 的字段协议、根因枚举及稳定排序规则。 -->
# Analyze 输出

## `analysis.json` 输出协议

`actrail-kv-analyze` 的输出是一个 JSON object。

- 字段路径采用 API 参考写法：`[]` 表示数组成员，`?` 表示该字段可省略。
- object 与 array 本身也有字段行，说明其职责和消费方式。
- 消费者使用 `run.schema_version` 选择解析逻辑。
- `top_k[]` 是当前实现给出的行动优先级列表；调用方无需按请求对重新计算排名。

如果你刚开始使用本工具，先阅读[请求模板概念](../../concepts/template.md)，再查阅以下字段协议。
| 字段路径 | 类型 | 含义 |
|---|---|---|
| `run` | object | 本次分析运行摘要、输入处理结果、完整参数快照与解释限制。 |
| `run.schema_version` | string | 产物 schema 版本，供下游兼容性判断。 |
| `run.input_records` | integer | 输入 NDJSON 中读取的记录总数。 |
| `run.analyzed_records` | integer | 成功投影并进入分析的记录数。 |
| `run.skipped_records[]` | object array | 未参与分析的输入记录及其原因。 |
| `run.skipped_records[].record_index` | integer | 未分析记录在输入 NDJSON 中的一基行号。 |
| `run.skipped_records[].reason` | string | 未分析原因。 |
| `run.options` | object | 本次实际生效的算法阈值和资源预算快照；不可假定等同于未来运行的默认值。 |
| `run.options.top_k` | integer | 本次产物请求的 Top K 数量。 |
| `run.options.min_template_members` | integer | 抽取模板所需的最小成员数。 |
| `run.options.stable_span_support_ratio` | number | stable span 最小成员支持率。 |
| `run.options.min_stable_support` | integer | stable span 最小成员支持数。 |
| `run.options.min_blocked_stable_bytes` | integer | 产生 Finding 所需的最小被阻断稳定 UTF-8 bytes。 |
| `run.options.min_exact_anchor_bytes` | integer | 恢复稳定内容所需的最小精确锚点 UTF-8 bytes。 |
| `run.options.text_similarity_threshold` | number | 有界文本 shingle 相似度阈值。 |
| `run.options.template_compatibility_threshold` | number | 候选请求加入同一模板的结构兼容阈值。 |
| `run.options.max_dynamic_coverage_ratio` | number | 模板允许的最大动态区域比例。 |
| `run.options.max_candidates_per_request` | integer | 单请求候选召回上限。 |
| `run.options.max_projection_units` | integer | 单请求投影单元上限。 |
| `run.options.max_text_unit_bytes` | integer | 单文本单元 UTF-8 byte 上限。 |
| `run.options.max_payload_bytes` | integer | 单 payload byte 上限。 |
| `run.options.max_alignment_cells` | integer | 单次序列对齐 cell 上限。 |
| `run.options.max_total_alignment_cells` | integer | 单 cohort 累计对齐 cell 上限。 |
| `run.options.max_records` | integer | 单次分析可读取的记录数上限。 |
| `run.limitations?[]` | string | 影响解释范围的说明。 |
| `templates[]` | object array | 以相同比较域和相近结构聚合的请求模板。 |
| `templates[].id` | string | 模板稳定 ID；由 `findings[].template_id` 关联。 |
| `templates[].dialect` | string | 请求 adapter/dialect。 |
| `templates[].model` | string | 模型名；不同 model 不进入同一模板。 |
| `templates[].adapter_revision` | string | adapter 修订号；不同修订号不进入同一模板。 |
| `templates[].member_request_ids[]` | string | 模板成员的稳定请求 ID。 |
| `templates[].medoid_request_id` | string | 确定性对齐基准的代表请求 ID。 |
| `templates[].cohesion` | number，`0..1` | 模板成员的结构一致性；越高越紧凑。 |
| `templates[].stable_spans[]` | object array | 模板成员共同支持的连续稳定片段。 |
| `templates[].stable_spans[].source` | object | 稳定片段在原始 payload 中的定位信息。 |
| `templates[].stable_spans[].source.json_path` | string | 原 payload 的 JSONPath。 |
| `templates[].stable_spans[].source.unit_index?` | integer | 投影单元序号。 |
| `templates[].stable_spans[].source.byte_start?` | integer | 原 UTF-8 字符串中片段的起始 byte offset。 |
| `templates[].stable_spans[].source.byte_end?` | integer | 原 UTF-8 字符串中片段的排他结束 byte offset。 |
| `templates[].stable_spans[].source.role?` | string | chat message role。 |
| `templates[].stable_spans[].utf8_bytes` | integer | 稳定片段长度。 |
| `templates[].stable_spans[].support_count` | integer | 支持该片段的模板成员数。 |
| `templates[].stable_spans[].excerpt?` | string | 用于人工核验的稳定片段摘录。 |
| `templates[].slots[]` | object array | 模板内的动态区域及其变体统计。 |
| `templates[].slots[].source` | object | 动态槽位在原始 payload 中的定位信息。 |
| `templates[].slots[].source.json_path` | string | 原 payload 的 JSONPath。 |
| `templates[].slots[].source.unit_index?` | integer | 投影单元序号。 |
| `templates[].slots[].source.byte_start?` | integer | 原 UTF-8 字符串中槽位的起始 byte offset。 |
| `templates[].slots[].source.byte_end?` | integer | 原 UTF-8 字符串中槽位的排他结束 byte offset。 |
| `templates[].slots[].source.role?` | string | chat message role。 |
| `templates[].slots[].member_count` | integer | 覆盖该槽位的模板成员数。 |
| `templates[].slots[].distinct_variant_count` | integer | 该槽位观察到的不同取值数量。 |
| `templates[].slots[].confidence` | number，`0..1` | 槽位识别置信度。 |
| `findings[]` | object array | 所有已聚合的结构优化机会；不受 `--top-k` 截断。 |
| `findings[].id` | string | Finding 稳定 ID；由 `top_k[].finding_id` 引用。 |
| `findings[].template_id` | string | 所属 `templates[].id`。 |
| `findings[].cause` | string enum | 根因；允许值见下表。 |
| `findings[].source` | object | 首个可归因分歧在原始 payload 中的定位信息。 |
| `findings[].source.json_path` | string | 原 payload 的 JSONPath，定位首个可归因分歧。 |
| `findings[].source.unit_index?` | integer | 投影单元序号。 |
| `findings[].source.byte_start?` | integer | 原 UTF-8 字符串中分歧的起始 byte offset。 |
| `findings[].source.byte_end?` | integer | 原 UTF-8 字符串中分歧的排他结束 byte offset。 |
| `findings[].source.role?` | string | chat message role。 |
| `findings[].actual_prefix_bytes` | integer | 当前组织下、分歧前可观察公共前缀的 UTF-8 bytes。 |
| `findings[].potential_prefix_bytes` | integer | 安全反事实成立时可能达到的公共前缀 UTF-8 bytes。 |
| `findings[].blocked_stable_bytes` | integer | 被阻断且有成员支持的稳定 UTF-8 bytes；不是 token 数。 |
| `findings[].affected_count` | integer | 受影响请求数，即模板成员数减最大同变体频次。 |
| `findings[].confidence` | number，`0..1` | 投影可靠度、cohesion、支持率及归因确定性的组合。 |
| `findings[].score` | object | 可解释的排名因子及最终得分。 |
| `findings[].score.blocked_stable_bytes` | integer | 排名时使用的被阻断稳定 bytes 快照。 |
| `findings[].score.affected_count` | integer | 排名时使用的受影响请求数快照。 |
| `findings[].score.confidence` | number，`0..1` | 排名时使用的置信度快照。 |
| `findings[].score.score` | number | `blocked_stable_bytes × affected_count × confidence`。 |
| `findings[].evidence` | object | 供人工复核的代表请求、差异片段和被阻断稳定片段。 |
| `findings[].evidence.representative_request_ids[]` | string | 形成该结论的代表请求 ID。 |
| `findings[].evidence.divergent_excerpts[]` | string | 差异片段；可能含敏感 payload 内容。 |
| `findings[].evidence.blocked_stable_excerpt` | string | 被差异阻断的稳定片段。 |
| `findings[].counterfactual` | string | 用于计算 `potential_prefix_bytes` 的单一可解释假设。 |
| `findings[].recommendation` | string | 建议的请求组织调整；工具不会自动执行。 |
| `top_k[]` | object array | `findings[]` 的稳定排序前缀，供优先行动。 |
| `top_k[].rank` | integer | 一基行动优先级名次。 |
| `top_k[].finding_id` | string | 对应 `findings[].id`。 |
| `top_k[].score` | object | 关联 Finding 的排名因子与最终得分快照。 |
| `top_k[].score.blocked_stable_bytes` | integer | 排名时使用的被阻断稳定 bytes 快照。 |
| `top_k[].score.affected_count` | integer | 排名时使用的受影响请求数快照。 |
| `top_k[].score.confidence` | number，`0..1` | 排名时使用的置信度快照。 |
| `top_k[].score.score` | number | 排名最终得分。 |

所有 `byte_start` 与 `byte_end` 都落在合法 UTF-8 字符边界，`byte_end` 为排他边界。所有 `json_path` 都从原始、未经重写的 payload 定位。

## `findings[].cause` 枚举

| 值 | 含义 |
|---|---|
| `early_variable_content` | 易变内容在稳定上下文之前。 |
| `inline_dynamic_slot` | 同一文本单元的中间动态槽阻断后续稳定文本。 |
| `dynamic_block_before_stable` | 独立动态块位于稳定块之前。 |
| `tool_order_drift` | 内容可一一对应的工具定义顺序不一致。 |
| `tool_definition_drift` | 同一工具定义的模型可见内容发生漂移。 |
| `system_prompt_drift` | system prompt 的稳定内容发生漂移。 |
| `non_append_only_history` | 对话历史存在非追加式插入或改动。 |
| `model_visible_format_drift` | 模型可见 JSON 文本语义等价但格式不同。 |

## 完整示例

```json
{
  "run": {
    "schema_version": "0.1.0",
    "input_records": 4,
    "analyzed_records": 4,
    "skipped_records": [],
    "options": {
      "top_k": 20, "min_template_members": 3, "stable_span_support_ratio": 0.8,
      "min_stable_support": 3, "min_blocked_stable_bytes": 64, "min_exact_anchor_bytes": 24,
      "text_similarity_threshold": 0.8, "template_compatibility_threshold": 0.68,
      "max_dynamic_coverage_ratio": 0.35, "max_candidates_per_request": 128,
      "max_projection_units": 512, "max_text_unit_bytes": 1048576,
      "max_payload_bytes": 8388608, "max_alignment_cells": 2000000,
      "max_total_alignment_cells": 64000000, "max_records": 1000000
    },
    "limitations": ["Structural proxy only; no tokenizer or KV telemetry is available."]
  },
  "templates": [{
    "id": "tpl-6cba...", "dialect": "openai_chat", "model": "gpt-example", "adapter_revision": "1",
    "member_request_ids": ["req-a", "req-b", "req-c", "req-d"], "medoid_request_id": "req-a", "cohesion": 0.91,
    "stable_spans": [{"source": {"json_path": "$.messages[1].content", "unit_index": 3, "byte_start": 0, "byte_end": 128, "role": "system"}, "utf8_bytes": 128, "support_count": 4, "excerpt": "Keep the following policy unchanged..."}],
    "slots": [{"source": {"json_path": "$.messages[0].content", "unit_index": 1, "byte_start": 0, "byte_end": 14, "role": "system"}, "member_count": 4, "distinct_variant_count": 4, "confidence": 0.91}]
  }],
  "findings": [{
    "id": "finding-184a...", "template_id": "tpl-6cba...", "cause": "early_variable_content",
    "source": {"json_path": "$.messages[0].content", "unit_index": 1, "byte_start": 0, "byte_end": 14, "role": "system"},
    "actual_prefix_bytes": 0, "potential_prefix_bytes": 128, "blocked_stable_bytes": 128, "affected_count": 3, "confidence": 0.91,
    "score": {"blocked_stable_bytes": 128, "affected_count": 3, "confidence": 0.91, "score": 349.44},
    "evidence": {"representative_request_ids": ["req-a", "req-b"], "divergent_excerpts": ["tenant: north", "tenant: south"], "blocked_stable_excerpt": "Keep the following policy unchanged..."},
    "counterfactual": "Move the variable system content after the stable instruction.",
    "recommendation": "Emit stable system instructions before tenant-specific content."
  }],
  "top_k": [{"rank": 1, "finding_id": "finding-184a...", "score": {"blocked_stable_bytes": 128, "affected_count": 3, "confidence": 0.91, "score": 349.44}}]
}
```

## 排序契约

- `affected_count` 表示“有多少请求落在这个问题的非主流分组中”。同一模板的请求会分成“未出现该问题”和“每一种差异变体”这些组；人数最多的一组只作为计数参照，其余各组的请求数相加就是 `affected_count`。这不判断哪种组织方式正确，只避免两两比较把同一问题重复放大。
- `blocked_stable_bytes` 表示“本来能被复用、却被前面差异挡住的稳定内容有多长”。它按 UTF-8 bytes 计算，是结构代理，不是模型 token 数。
- 排名先看综合分 `score.score`：被挡住的稳定内容越长、受影响请求越多、判断越有把握，分数越高。分数相同时，依次比较受影响请求数和被挡住的稳定内容长度。
- 前三项仍相同时，使用稳定 ID 决定顺序。因此无论输入文件中请求行如何调换，`findings[]`、`top_k[]` 和最终 JSON 的顺序都保持一致。
- 工具顺序问题只在每个工具都能被唯一对应时才会输出，避免把两个相同工具误判为顺序变化。
- 格式漂移只在模型可见文本表示同一份 JSON 数据、但书写格式不同时才会输出；JSON 数组顺序被视为有意义，不会被当作纯格式差异。
