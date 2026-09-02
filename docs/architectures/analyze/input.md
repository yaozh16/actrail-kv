<!-- 本文件定义 actrail-kv-analyze 读取的 requests.ndjson 及可分析 payload 边界。 -->
# Analyze 输入

analyzer 读取 [Receiver 输出](../receiver/output.md) 的 NDJSON。它逐行处理，坏行、资源超限记录或无法投影的 payload 不会中断整批分析，而会写入 `analysis.json` 的 `run.skipped_records[]`。

| 输入路径 | 类型 | 要求 |
|---|---|---|
| `captured_at` | string，可缺失 | 采集元数据，参与请求身份但不进入上下文投影。 |
| `source` | string，可缺失 | 采集元数据，参与请求身份但不进入上下文投影。 |
| `payload` | JSON object，必填 | 待分析的完整模型请求。 |
| `payload.model` | non-empty string，必填 | 比较域的一部分。 |
| `payload.messages` | array，必填 | 依数组顺序投影；每个元素必须是 JSON object。 |
| `payload.tools?` | array | 按模型可见工具定义投影。 |

当前 OpenAI-compatible adapter 会按 `tools`、message role，以及 `name`、`content`、`function_call`、`tool_calls`、`tool_call_id`、`refusal` 的逻辑顺序投影。未知模型字段不会自动作为模型可见上下文加入分析。

比较域为 `(dialect, model, adapter_revision)`；不同域绝不聚类。HTTP 外层 object key 顺序和空白不进入投影；模型可见字符串保留原始 UTF-8，非字符串模型可见字段使用 canonical JSON 结构代理。
