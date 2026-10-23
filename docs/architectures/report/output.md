<!-- 本文件定义 actrail-kv-report 的 HTML 与 GFM Markdown 双产物契约。 -->
# Report 输出

`actrail-kv-report` 从同一份 `analysis.json` 生成 HTML 和/或纯 GFM Markdown：

| `--format` | 产物 |
|---|---|
| 不传 | 同名 `report.html` + `report.md` |
| `html` | 仅写入 `--output` 指定的 HTML 路径 |
| `md` | 仅写入 `--output` 指定的 Markdown 路径 |

默认双份模式会把 `.html`、`.md` 或无后缀输出路径归一化为同一基础路径。HTML 是单文件、
无脚本静态页面；Markdown 不嵌入 HTML、CSS、SVG 或 JavaScript。

`--locale` 默认 `zh-CN`，首期支持 `zh-CN`、`en-US`。一次导出固定一个 locale 快照，
HTML 与 Markdown 使用相同语言。HTML 的 `lang` 属性随 locale 变化。

| 区域 | 数据来源 | 展示内容 |
|---|---|---|
| 标题与摘要 | `run`、`defects[]` | 输入、分析、跳过记录数和聚合问题数。 |
| 结构复用率 | `prefix_reuse` | 首屏刻度条：当前复用率、补完已知结构问题后的上界、以及 90% 目标；下方给出"已复用 / 已知可恢复 / 每轮新增"三段字节构成与结论句。字段缺省时整块不渲染。 |
| Top K 表格 | `top_k[]`、`defects[]` | 名次、位置（JSONPath + role + 投影单元序号）、被阻断稳定字节、受影响请求数、得分、建议动作与跳转到证据的锚点。 |
| 缺陷详情 | `defects[]` | comparison group、P1、X 变体与事实、P2 恢复证据、指标和中文优化启示；每个缺陷带锚点，供清单直接跳入。 |
| 会话上下文刷新与前缀复用 | `session_reports[]` | 概览（会话数、请求数、刷新 warning 数、受影响会话数、废弃前缀字节）、刷新 warning 会话表（含首次刷新轮次）、其余会话明细与逐事件折叠表；逐请求展示前缀复用度和废弃字节。 |
| KV 缓存命中率 | `cache_metrics[]` | 逐请求 `prompt` / `hit(cached)` / `miss` / `output`、首 token 时延与总耗时、命中率与环比变化；按 `basis` 标注“真实”或“估算”。时延是上游观测值，未上报时显示 `—`。 |
| 结构代理 vs 真实命中 | `prefix_reuse.by_session`、`cache_metrics[]` | 逐会话并列结构复用率（字节口径）与真实命中率（token 口径，按真实样本加权）及差值。 |
| 模板前缀树 | `templates[].prefix_view` | 复用分布、成员衰减、结构片段列表（类型、摘要、字节数、支持度）与可展开的字段明细、变体、缺陷锚点。字段缺省时整块不渲染。 |

## 展示契约

- Top K 表格保留原名称与排序：名次即 `top_k[]` 的稳定序（score → affected → blocked → id），报告不重新排序、不使用第二种排序口径；相比早期版本只增加了位置、建议动作与证据锚点三列。
- 90% 结构复用率目标是报告层的产品参数，写死在 `actrail-kv-report` 中（`PREFIX_REUSE_TARGET`），不是 `analysis.json` 字段，也不是配置项。
- 结构复用率与结构上界都是**字节口径的结构代理**，不代表真实 token、KV miss、延迟或金额收益；只有 `cache_metrics.basis = reported` 的计数来自上游 usage。
- 结构代理与真实命中的对照只用于判断方向是否一致，两者口径不同，不表示应当相等；`basis = estimated` 的请求不参与对照，也不得被展示为真实命中率。
- 会话刷新是 warning，不是 `defects[]` 中的错误；compact、subagent 与任务切换都可能产生刷新，由用户判断是否接受。
- `prefix_reuse.ratio` 表示“当前请求有多少前缀可复用”，会话事件中的 `prefix_reuse_ratio` 表示“上一条上下文被保留了多少”，两者口径不同。
- 所有可选输入字段缺省时，对应区块整块不渲染，旧版 `analysis.json` 仍可生成报告。
- HTML 与 Markdown 必须保持信息内容等价；Markdown 不复刻 CSS、颜色、进度条或交互式折叠。
- HTML 中的所有文本经过 HTML escape；Markdown 中的普通文本经过 GFM escape，代码片段使用动态长度围栏。
- Markdown 中原 HTML 的复用率色条、逐轮走势、成员衰减 SVG 和折叠事件分别转换为指标表、逐轮列表、
  成员衰减表和事件表，不丢失原始数值。
- locale 只翻译固定 UI 文案、表头、说明和事件标签；请求内容、ID、路径、技术标识和
  analyzer `analysis.json` insights 保持原文。
- 两种产物都不包含远程资源或请求自动改写行为。
