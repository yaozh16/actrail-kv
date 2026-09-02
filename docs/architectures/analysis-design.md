<!-- 本文件定义 v0.1.0 结构诊断的实现契约，不取代用户可配置项参考。 -->
# 分析实现设计

## 交付边界

| 二进制 | 输入 | 输出 | 职责 |
|---|---|---|---|
| `actrail-kv-receiver` | `POST /requests` 的完整 JSON object | `requests.ndjson` | 补采集时间并安全追加；不解释模型语义。 |
| `actrail-kv-analyze` | `requests.ndjson` | `analysis.json` | 离线投影、模板抽取、结构诊断、聚合及 Top K。 |
| `actrail-kv-report` | `analysis.json` | `report.html` | 安全转义并渲染静态报告；不重新计算分析。 |

运行时不访问外部网络、数据库、模型服务或 tokenizer。结论只描述结构风险和优化机会，不声称真实 KV miss、Token 或金额收益。

## 输入与比较域

`CapturedRequest` 保存 `captured_at`、可选 `source` 与未经改写的 `payload`。OpenAI-compatible adapter 把 `model`、`tools`、`messages` 及模型可见文本投影成有序 `CacheSequence`；未知结构显式 skipped。比较域是 `(dialect, model, adapter_revision)`，不同域永不聚类。

投影只声明模型可见结构、逻辑顺序、单元类型、角色、工具稳定身份和源 JSON path，不识别业务值。HTTP 外层对象键序与空白不进入投影；模型可见字符串保留原始 UTF-8。

## 模板抽取与诊断

1. 加载 NDJSON、执行资源预算并建立比较域。
2. 将每条请求投影为可回溯 JSON path 与 UTF-8 byte range 的 `CacheSequence`。
3. 以结构签名、稳定块签名、文本 shingle、工具无序集合召回有界候选。
4. 用确定性 medoid 星型分簇，避免相似图传递闭包产生 bridge 合并。
5. 对齐 unit；对应文本 unit 做 byte-safe diff；工具重排按唯一稳定身份一一对应。
6. 支持率达标的连续对齐区域成为 stable span，其余局部区域成为 slot。
7. 只有首个分歧之后仍有达标 stable span 才形成机会；正常动态后缀被抑制。
8. 每个候选只采用一个可解释反事实，计算可观察 actual/potential prefix；无法恢复稳定内容不报告。
9. 以比较域、模板、原因、归一化位置、稳定锚点与建议生成 fingerprint，先聚合根因再排序。

算法不得借助字段业务名、UUID/时间戳正则或固定业务词汇决定模板或原因；业务值只进入证据。

## Finding 契约与排序

每个 Finding 包含稳定 ID、模板 ID、cause、source location、actual/potential prefix bytes、blocked stable bytes、affected count、confidence、得分明细、代表证据、反事实说明和建议。UTF-8 offset 必须能回切原字符串且在合法字符边界。

- `affected = support_count - largest_variant_frequency`，禁止使用请求 pair 数。
- `blocked_bytes` 是受支持成员可恢复稳定字节数的保守值。
- `score = blocked_bytes × affected × confidence`。
- 排序为 score、affected、blocked bytes 降序，fingerprint 升序。
- 输入行置换不能影响模板、Finding、Top K 或最终 JSON 字节。

工具顺序只在工具稳定身份唯一且内容可一一对应时报告。格式漂移只在完整模型可见文本均可解析为严格等价 JSON AST、原始表示不同时报告；数组顺序保持语义。

## 原子性与安全

receiver 的单写入临界区保证每请求一条完整 NDJSON；非法 JSON 或非 object 返回 400。analyze 与 report 采用同目录临时文件和原子 rename，禁止 input/output 指向同一文件或硬链接。报告中所有请求证据必须 HTML escape。
