<!-- 本文件定义 actrail-kv-report 接受的 analysis.json 输入契约。 -->
# Report 输入

reporter 只读取 analyzer 生成的 `analysis.json`，不重新分析请求。完整字段协议见 [Analyze 输出](../analyze/output.md)。

| 校验路径 | 要求 |
|---|---|
| `run.schema_version` | 必须等于当前 artifact schema。 |
| `run.options` | Top K、时间窗口、固定版本阈值和资源预算必须大于零；模板/稳定支持数至少为二；所有比例必须为 `0..1` 内的有限数。 |
| `templates[].id` | 必须唯一。 |
| `defects[].id` | 必须唯一。 |
| `defects[].template_id` | 必须引用已存在的 `templates[].id`。 |
| `defects[].mismatch.variants[]` | 至少两个；成员不重叠、代表请求属于自身变体，成员总数等于 `comparable_count`。 |
| `defects[].recovered_stable` | sources 非空，support count 等于 `comparable_count`。 |
| `defects[].score` | 必须与 defect 指标及评分公式完全一致。 |
| `top_k[]` | 不重复的 defect ID，必须是 `defects[]` 稳定排序的前缀。 |
| `conditional_local_sites[]` | ID 唯一，引用已有模板；每个模板的 episode index 必须从 2 连续递增、不得留空洞，局部字节关系、成员和证据必须自洽，且不得进入 `top_k`。 |
| `session_analysis.timelines[]` | Session ID 唯一；请求 ID 全局唯一；transition 恰好连接所有相邻请求，时间字段、排序和分类必须一致。 |
| 可比较 Session transition | `preserved_prefix_bytes` 不超过前后请求字节数；保留比例和失效旧后缀必须由字节字段精确导出。 |
| `session_analysis.timelines[].transitions[].boundary?` | 可比较 outcome 必须携带完整非空边界；不可比、歧义和不可分析 outcome 不得携带。 |
| `session_analysis.history_sites[]` | ID 唯一；只能引用同一种历史变化或前缀截断 transition，kind、边界和逻辑位置必须与所有引用一致，聚合次数、Session 数和失效字节统计必须一致。 |

所有新增 source location 继续要求非空 JSON path 和成对合法 byte range。任何引用、枚举或数值关系不满足契约时 report 命令失败，不生成伪完整 HTML。
