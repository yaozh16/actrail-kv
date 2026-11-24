<!-- 本文件定义采集记录及 NDJSON 的行级协议。 -->
# Receiver 输出

`requests.ndjson` 为 UTF-8 NDJSON：每一物理行恰好是一条成功接收的请求记录。字段路径以 `[]` 表示数组成员，`?` 表示可省略字段。

| 字段路径 | 类型 | 含义 |
|---|---|---|
| `captured_at` | string | receiver 以 UTC 写入的 RFC 3339 采集时间。 |
| `source?` | string | 入站 `X-Actrail-Source` 的原样值。 |
| `session_key?` | string | 入站 `X-Actrail-Session-Key` 的原样值；缺失表示未知会话。 |
| `comparison` | object | analyzer 建立 comparison group 所需的显式采集元数据。 |
| `comparison.endpoint_key` | string | LLM endpoint 或逻辑路由标识。 |
| `comparison.agent_key?` | string | 可选 Agent 标识。 |
| `comparison.model_deployment_key?` | string | 可选模型部署标识。 |
| `comparison.kv_namespace?` | string | 可选真实 KV 隔离 namespace。 |
| `payload` | JSON object | 原始 HTTP body；receiver 不改写其字段和值。 |

```json
{"captured_at":"2026-09-02T08:09:10Z","source":"agent-hook","comparison":{"endpoint_key":"llm-primary","agent_key":"coding-agent"},"payload":{"model":"example","messages":[{"role":"user","content":"你好"}]}}
```

`--output` 模式由一个 Receiver 进程独占文件。托管 NDJSON 模式按时间和大小轮转，一条记录始终完整写入一个分片。Analyzer 和导出器通过同一存储接口遍历已配置的后端。见[采集存储架构](../storage/index.md)。
