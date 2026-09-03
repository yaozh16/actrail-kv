<!-- 本文件定义 actrail-kv-report 产物 report.html 的展示契约。 -->
# Report 输出

`report.html` 是单文件、无脚本的静态 HTML，用于内网人工审阅。

| 区域 | 数据来源 | 展示内容 |
|---|---|---|
| 标题与摘要 | `run`、`defects[]` | 输入、分析、跳过记录数和聚合问题数。 |
| Top K 表格 | `top_k[]`、`defects[]` | 名次、Defect ID、得分、受影响请求数与被阻断稳定 bytes。 |
| 诊断详情 | `defects[]` | comparison group、P1、X 变体与事实、P2 恢复证据、指标和中文优化启示。 |
| 条件性局部位点 | `conditional_local_sites[]` | 按模板展示 direct defect 之后的局部 `X → P2` 机会，明确不进入 Top K 且不可与其他位点累计。 |
| Session 摘要 | `session_analysis` | 提供 Session 的记录数、时间线数、transition 分类计数和 history site 数。 |
| Session 时间线 | `session_analysis.timelines[]` | 相邻请求、采集时间、分类、可观察字节指标、首次分叉证据和可比较边界。 |
| 历史变化位点 | `session_analysis.history_sites[]` | 变化/截断类别、比较边界、重复逻辑位置、发生次数、受影响 Session 数、失效旧后缀统计和可用建议。 |

没有 Session ID 或没有可比较相邻请求时，报告明确显示“未提供 Session ID 或没有可分析的时间线”，而不是显示为零历史问题。所有来自 `analysis.json` 的文本——包括 Session ID、请求 ID、source、JSON path、证据摘录与建议——均经过 HTML escape。该产物不包含 JavaScript、远程资源或请求自动改写行为。
