<!-- 本文件定义 actrail-kv-report 接受的 analysis.json 输入契约。 -->
# Report 输入

reporter 只读取 analyzer 生成的 `analysis.json`，不重新分析请求。完整字段协议见 [Analyze 输出](../analyze/output.md)。

| 校验路径 | 要求 |
|---|---|
| `run.schema_version` | 当前必须为 `0.1.0`。 |
| `templates[].id` | 必须唯一。 |
| `findings[].id` | 必须唯一。 |
| `findings[].template_id` | 必须引用已存在的 `templates[].id`。 |
| `top_k[].rank` | 必须从 `1` 连续递增。 |
| `top_k[].finding_id` | 必须引用已存在的 `findings[].id`，不得重复。 |
| `top_k[].score` | 必须与被引用 Finding 的 score 完全一致。 |

不满足任一项时 report 命令失败，不生成伪完整 HTML。
