<!-- 本文件定义 actrail-kv-report 接受的 analysis.json 输入契约。 -->
# Report 输入

reporter 只读取 analyzer 生成的 `analysis.json`，不重新分析请求。完整字段协议见 [Analyze 输出](../analyze/output.md)。

| 校验路径 | 要求 |
|---|---|
| `run.schema_version` | 当前必须为 `0.1.0`。 |
| `templates[].id` | 必须唯一。 |
| `defects[].id` | 必须唯一。 |
| `defects[].template_id` | 必须引用已存在的 `templates[].id`。 |
| `defects[].mismatch.variants[]` | 至少两个；成员不重叠、代表请求属于自身变体，成员总数等于 `comparable_count`。 |
| `defects[].recovered_stable` | sources 非空，support count 等于 `comparable_count`。 |
| `defects[].score` | 必须与 defect 指标及评分公式完全一致。 |
| `top_k[]` | 不重复的 defect ID，必须是 `defects[]` 稳定排序的前缀。 |
| `prefix_reuse` / `cache_metrics[]` / `templates[].prefix_view` | 可选字段，不参与强校验；缺省时对应报告区块整块不渲染。 |

不满足任一项时 report 命令失败，不生成伪完整 HTML。
