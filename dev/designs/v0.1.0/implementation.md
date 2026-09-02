<!-- 本文件固定 v0.1.0 的实现边界、算法契约、数据契约与发布验收门槛。 -->
# LLM KV 结构诊断 MVP 实现设计

## 设计索引

- 对应需求：`dev/requirements/v0.1.0/llm-kv-structure-diagnostics-mvp.md`
- 对应布局：`docs/architectures/code-layout.md`
- 当前决策：Rust workspace，三个独立二进制，共享序列化 artifact。
- 失效决策：无。此前 code layout 的语言无关描述仍有效，本记录只补充 Rust 映射。

## 交付边界

| 二进制 | 输入 | 输出 | 职责 |
|---|---|---|---|
| `actrail-kv-receiver` | `POST /requests` 的完整 JSON object | `requests.ndjson` | 补采集时间并并发安全地整行追加；不做模型语义分析 |
| `actrail-kv-analyze` | `requests.ndjson` | `analysis.json` | 离线完成投影、模板抽取、结构诊断、聚合和 Top K |
| `actrail-kv-report` | `analysis.json` | `report.html` | 安全转义并渲染静态报告；不重新计算分析结果 |

运行时不得访问外部网络、数据库、模型服务或 tokenizer。分析结果只描述结构风险和优化机会，不声称真实 KV miss、Token 或金额收益。

## 输入与比较域

`CapturedRequest` 保存 `captured_at`、可选 `source` 和未经改写的 `payload`。receiver 接收的 body 即裸 payload；可选 source 只来自请求头并作为采集证据，不参与算法规则。

analyzer 通过 dialect adapter 将 payload 投影为有序 `CacheSequence`。MVP 支持 OpenAI-compatible chat payload：`model`、`tools`、`messages` 及模型可见文本；未知结构显式记为 skipped。比较域键为 `(dialect, model, adapter_revision)`，不同域永不聚类。

adapter 只声明模型可见结构、逻辑顺序、单元类型、角色、工具稳定身份和源 JSON path，不识别业务值。HTTP 外层 JSON 对象键序与空白不进入投影；模型可见字符串保留原始 UTF-8 表示。

## 模板抽取与诊断流水线

固定顺序为：

1. 加载 NDJSON、校验资源预算、建立比较域。
2. 投影 `CacheSequence` 并保留 JSON path 与 UTF-8 byte range。
3. 由结构签名、稳定块签名、文本 shingle 和工具无序集合签名召回候选；每请求候选数有上限。
4. 使用确定性 medoid 星型分簇，禁止相似图传递闭包导致模板链式误合并。
5. unit 层执行确定性序列对齐；对应文本 unit 执行 byte-safe 文本 diff；工具重排另做稳定身份一一对应。
6. 跨成员支持率达到阈值的对齐区域成为 stable span，其余局部区域合并为 slot。
7. 仅当首个分歧之后仍存在达到门槛的 stable span 时诊断优化机会；纯动态后缀抑制。
8. 每个候选只应用一个可解释反事实，计算可观察 actual/potential prefix；不能恢复稳定内容则不报告。
9. 按比较域、模板、原因、归一化位置、稳定锚点和建议生成 fingerprint，先聚合根因再排序。

实现不得用字段业务名、UUID/时间戳正则或固定业务词汇决定模板或原因。业务值只进入证据。

## 诊断证据

必须支持需求表中的八类正例：早期易变内容、文本内部动态槽、动态块前置、工具顺序漂移、工具定义微漂移、system prompt 微漂移、非追加式历史、模型可见格式漂移。

每个 Finding 至少包含稳定 ID、模板 ID、cause、source location、actual/potential prefix bytes、blocked stable bytes、affected count、confidence、score breakdown、代表证据、反事实说明和建议。UTF-8 offset 必须能切回原始字符串且位于合法字符边界。

工具顺序结论要求工具稳定身份唯一且内容能够一一对应；歧义时不得报告。格式漂移要求完整模型可见文本均能解析为 JSON、语法树严格等价且原始表示不同；数组顺序保留语义。

## 默认阈值与资源预算

阈值保存在 `AnalysisOptions` 并完整写入 analysis run：

- 最小模板成员数 `3`；stable span 支持率 `0.80` 且支持数不少于 `3`。
- 最小被阻断稳定内容 `64` UTF-8 bytes；最小精确锚点 `24` bytes。
- 局部文本相似度 `0.80`；模板兼容度 `0.68`；最大动态覆盖率 `0.35`。
- 每请求最多 `128` 个候选；最多 `512` 个投影 unit；单文本 unit 最多 `1 MiB`；单 payload 最多 `8 MiB`；单次对齐最多 `2,000,000` cells。
- 单个 cohort 的 unit/text 对齐累计最多 `64,000,000` cells；候选文本相似度使用有界 byte shingle，不执行无界编辑距离。

超限必须 skipped 或 degraded 并给出原因，不得静默截断后输出高置信结论。

## 聚合与排序

- `affected = support_count - largest_variant_frequency`，不得使用请求 pair 数。
- `blocked_bytes` 使用受支持成员可恢复稳定字节数的保守值。
- `confidence` 由投影可靠度、模板 cohesion、stable span support 和归因确定性组成。
- `score = blocked_bytes × affected × confidence`。
- 排序依次为 score、affected、blocked bytes 降序，fingerprint 升序；结果与输入行顺序无关。
- 同一模板、同一位置且能由同一修复解释的重叠问题只保留覆盖更完整的根因。

## 原子性与安全

receiver 对并发请求使用单写入临界区，保证每请求一条完整 NDJSON；非法 JSON/非 object 返回 400。analyze 和 report 使用同目录临时文件后原子 rename，失败不留下伪完整产物。报告中的一切请求证据必须 HTML escape。

## Ready 验收门槛

- 需求表八类正例全部召回，每类至少两套不同业务词汇和值形态。
- 正常动态后缀、不同模板、非上下文字段、跨比较域、HTTP 外层序列化差异全部零误报。
- 输入置换不变、重复运行确定、同根因只产生一个聚合 Finding、Top K 前缀稳定。
- Unicode source offset 可回溯；并发接收不坏行；恶意 HTML 证据被转义。
- 阈值边界、资源预算、空/坏输入、未知 dialect 均有测试。
- 三个真实二进制完成 receiver → analyze → report 端到端测试。
- `cargo fmt --check`、`cargo clippy -D warnings`、workspace debug/release tests 全部通过。
