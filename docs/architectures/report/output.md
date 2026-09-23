<!-- 本文件定义 actrail-kv-report 产物 report.html 的展示契约。 -->
# Report 输出

`report.html` 是单文件、无脚本的静态 HTML，用于内网人工审阅。

| 区域 | 数据来源 | 展示内容 |
|---|---|---|
| 标题与摘要 | `run`、`defects[]` | 输入、分析、跳过记录数和聚合问题数。 |
| 结构复用率 | `prefix_reuse` | 首屏刻度条：当前复用率、补完已知结构问题后的上界、以及 90% 目标；下方给出"已复用 / 已知可恢复 / 每轮新增"三段字节构成与结论句。字段缺省时整块不渲染。 |
| Top K 表格 | `top_k[]`、`defects[]` | 名次、位置（JSONPath + role + 投影单元序号）、被阻断稳定字节、受影响请求数、得分、建议动作与跳转到证据的锚点。 |
| 缺陷详情 | `defects[]` | comparison group、P1、X 变体与事实、P2 恢复证据、指标和中文优化启示；每个缺陷带锚点，供清单直接跳入。 |
| 会话前缀切换证据 | `session_reports[]` | 概览（会话数、请求数、复用前缀占比、存在前缀收缩的会话数、可恢复稳定字节）、需要关注的会话表、其余会话明细与逐事件折叠表；每行给出逐轮复用率走势。 |
| KV 缓存命中率 | `cache_metrics[]` | 逐请求 `prompt` / `hit(cached)` / `miss` / `output`、首 token 时延与总耗时、命中率与环比变化；按 `basis` 标注“真实”或“估算”。时延是上游观测值，未上报时显示 `—`。 |
| 结构代理 vs 真实命中 | `prefix_reuse.by_session`、`cache_metrics[]` | 逐会话并列结构复用率（字节口径）与真实命中率（token 口径，按真实样本加权）及差值。 |
| 模板前缀树 | `templates[].prefix_view` | 复用分布、成员衰减、结构片段列表（类型、摘要、字节数、支持度）与可展开的字段明细、变体、缺陷锚点。字段缺省时整块不渲染。 |

## 展示契约

- Top K 表格保留原名称与排序：名次即 `top_k[]` 的稳定序（score → affected → blocked → id），报告不重新排序、不使用第二种排序口径；相比早期版本只增加了位置、建议动作与证据锚点三列。
- 90% 结构复用率目标是报告层的产品参数，写死在 `actrail-kv-report` 中（`PREFIX_REUSE_TARGET`），不是 `analysis.json` 字段，也不是配置项。
- 结构复用率与结构上界都是**字节口径的结构代理**，不代表真实 token、KV miss、延迟或金额收益；只有 `cache_metrics.basis = reported` 的计数来自上游 usage。
- 结构代理与真实命中的对照只用于判断方向是否一致，两者口径不同，不表示应当相等；`basis = estimated` 的请求不参与对照，也不得被展示为真实命中率。
- 所有可选输入字段缺省时，对应区块整块不渲染，旧版 `analysis.json` 仍可生成报告。
所有来自 `analysis.json` 的文本均经过 HTML escape。该产物不包含 JavaScript、远程资源或请求自动改写行为。
