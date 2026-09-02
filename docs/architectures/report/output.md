<!-- 本文件定义 actrail-kv-report 产物 report.html 的展示契约。 -->
# Report 输出

`report.html` 是单文件、无脚本的静态 HTML，用于内网人工审阅。

| 区域 | 数据来源 | 展示内容 |
|---|---|---|
| 标题与摘要 | `run`、`findings` | 输入、分析、跳过记录数和聚合问题数。 |
| Top K 表格 | `top_k[]` | 名次、Finding ID、得分、受影响请求数与被阻断稳定 bytes。 |
| 诊断详情 | `findings[]` | 位置、原因、建议、反事实、前缀指标、差异证据和后续稳定证据。 |

所有来自 `analysis.json` 的文本均经过 HTML escape。该产物不包含 JavaScript、远程资源或请求自动改写行为。
