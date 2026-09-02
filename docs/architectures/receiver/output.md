<!-- 本文件定义 actrail-kv-receiver 产物 requests.ndjson 的行级协议。 -->
# Receiver 输出

`requests.ndjson` 为 UTF-8 NDJSON：每一物理行恰好是一条成功接收的请求记录。字段路径以 `[]` 表示数组成员，`?` 表示可省略字段。

| 字段路径 | 类型 | 含义 |
|---|---|---|
| `captured_at` | string | receiver 以 UTC 写入的 RFC 3339 采集时间。 |
| `source?` | string | 入站 `X-Actrail-Source` 的原样值。 |
| `comparison` | object | analyzer 建立 comparison group 所需的显式采集元数据。 |
| `comparison.endpoint_key` | string | LLM endpoint 或逻辑路由标识。 |
| `comparison.agent_key?` | string | 可选 Agent 标识。 |
| `comparison.model_deployment_key?` | string | 可选模型部署标识。 |
| `comparison.kv_namespace?` | string | 可选真实 KV 隔离 namespace。 |
| `payload` | JSON object | 原始 HTTP body；receiver 不改写其字段和值。 |

```json
{"captured_at":"2026-09-02T08:09:10Z","source":"agent-hook","comparison":{"endpoint_key":"llm-primary","agent_key":"coding-agent"},"payload":{"model":"example","messages":[{"role":"user","content":"你好"}]}}
```

该文件是 analyzer 的唯一输入格式。它由一个 receiver 进程独占；并发成功请求仍保证一请求一整行。
