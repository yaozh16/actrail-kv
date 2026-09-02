<!-- 本文件定义 actrail-kv-receiver 产物 requests.ndjson 的行级协议。 -->
# Receiver 输出

`requests.ndjson` 为 UTF-8 NDJSON：每一物理行恰好是一条成功接收的请求记录。字段路径以 `[]` 表示数组成员，`?` 表示可省略字段。

| 字段路径 | 类型 | 含义 |
|---|---|---|
| `captured_at` | string | receiver 以 UTC 写入的 RFC 3339 采集时间。 |
| `source?` | string | 入站 `X-Actrail-Source` 的原样值。 |
| `payload` | JSON object | 原始 HTTP body；receiver 不改写其字段和值。 |

```json
{"captured_at":"2026-09-02T08:09:10Z","source":"agent-hook","payload":{"model":"example","messages":[{"role":"user","content":"你好"}]}}
```

该文件是 analyzer 的唯一输入格式。它由一个 receiver 进程独占；并发成功请求仍保证一请求一整行。
