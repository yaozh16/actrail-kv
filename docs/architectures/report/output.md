<!-- 本文件定义 actrail-kv-report 产物 report.html 的展示契约。 -->
# Report 输出

`report.html` 是单文件、无脚本的静态 HTML，用于内网人工审阅。

| 区域 | 数据来源 | 展示内容 |
|---|---|---|
| 标题与摘要 | `run`、`defects[]` | 输入、分析、跳过记录数和聚合问题数。 |
| Top K 表格 | `top_k[]`、`defects[]` | 名次、Defect ID、得分、受影响请求数与被阻断稳定 bytes。 |
| 诊断详情 | `defects[]` | comparison group、P1、X 变体与事实、P2 恢复证据、指标和中文优化启示。 |
| 会话 Prefix-Switch 证据 | `session_reports[]` | 按会话给出的相邻请求公共前缀、切换类型、前缀收缩与可恢复稳定字节。 |
| KV 缓存命中率 | `cache_metrics[]` | 逐请求 `prompt` / `hit(cached)` / `miss` / `output`、首 token 时延与总耗时、命中率与环比变化；按 `basis` 标注“真实”或“估算”。时延是上游观测值，未上报时显示 `—`。 |

所有来自 `analysis.json` 的文本均经过 HTML escape。该产物不包含 JavaScript、远程资源或请求自动改写行为。
