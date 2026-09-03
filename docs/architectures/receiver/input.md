<!-- 本文件定义 actrail-kv-receiver 的 HTTP 输入契约。 -->
# Receiver 输入

## Endpoint

| 项 | 值 |
|---|---|
| Method | `POST` |
| Path | `/requests` |
| Body | 一个完整 JSON object，即采集方实际发往模型 API 的 request payload。 |
| 最大 body | `8 MiB` |

请求可来自网关、插桩组件或 Agent hook；receiver 不要求采集方类型，也不解析业务字段。

| 请求字段/头 | 类型 | 含义 |
|---|---|---|
| HTTP body | JSON object | 原始模型请求 payload。JSON array、scalar、空或非法 JSON 均被拒绝。 |
| `X-Actrail-Source?` | HTTP header string | 可选采集来源，例如 `agent-hook`；只保存证据，不参与模板与诊断。 |
| `X-Actrail-Endpoint-Key` | HTTP header string | 必填的 LLM endpoint 或逻辑路由标识，进入 comparison group。 |
| `X-Actrail-Agent-Key?` | HTTP header string | 可选 Agent 标识；提供后进入 comparison group。 |
| `X-Actrail-Model-Deployment-Key?` | HTTP header string | 可选模型部署标识；提供后进入 comparison group。 |
| `X-Actrail-KV-Namespace?` | HTTP header string | 可选真实 KV 隔离 namespace；提供后进入 comparison group。 |
| `X-Actrail-Session-Id?` | HTTP header string | 可选线性 Session 标识；只建立相邻请求时间线，不进入 payload 或 comparison group。 |

| 响应状态 | 条件 | Body |
|---|---|---|
| `202 Accepted` | 成功写入一条完整记录 | 空 |
| `400 Bad Request` | body 非合法 JSON object、缺少 endpoint key，或 metadata header 非有效非空文本 | 错误说明文本 |
| `405 Method Not Allowed` | 非 `POST /requests` | 框架默认响应 |
| `413 Payload Too Large` | body 超过 `8 MiB` | 框架默认响应 |
| `500 Internal Server Error` | 记录无法安全持久化 | `failed to persist captured request` |
