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
| `X-Actrail-Session-Key?` | HTTP header string | 可选会话标识；只做会话内 prefix-switch 分析，不进入 comparison group。 |
| `X-Actrail-Endpoint-Key` | HTTP header string | 必填的 LLM endpoint 或逻辑路由标识，进入 comparison group。 |
| `X-Actrail-Agent-Key?` | HTTP header string | 可选 Agent 标识；提供后进入 comparison group。 |
| `X-Actrail-Model-Deployment-Key?` | HTTP header string | 可选模型部署标识；提供后进入 comparison group。 |
| `X-Actrail-KV-Namespace?` | HTTP header string | 可选真实 KV 隔离 namespace；提供后进入 comparison group。 |

## payload 保留字段

采集方可以在请求 payload 里附加一个保留字段 `_actrail_response_usage`，用于上报该次请求的
上游响应观测值。receiver 不改写 payload，字段原样落盘；analyzer 解析后输出到 `cache_metrics[]`。
字段不参与模板抽取与缺陷判定（投影只读取 `model` / `messages` / `tools`）。

```json
{
  "model": "example",
  "messages": [{ "role": "user", "content": "你好" }],
  "_actrail_response_usage": {
    "prompt_tokens": 1200,
    "cached_tokens": 900,
    "completion_tokens": 50,
    "ttft_ms": 320,
    "total_ms": 1850
  }
}
```

| 字段 | 类型 | 含义 |
|---|---|---|
| `prompt_tokens` | integer | **总输入 token**，包含命中缓存的部分。 |
| `input_tokens` | integer | `prompt_tokens` 的等价写法，同样是总输入；与 `prompt_tokens` 二者取一。 |
| `cached_tokens` | integer | 命中缓存的输入 token，是总输入的子集。 |
| `completion_tokens` | integer | 输出 token。 |
| `output_tokens` | integer | `completion_tokens` 的等价写法；与 `completion_tokens` 二者取一。 |
| `ttft_ms` | integer | 首 token 时延（毫秒）：请求发出到首个内容或工具增量。 |
| `total_ms` | integer | 总耗时（毫秒），口径为 wall time：请求发出到流结束。 |

约定：

- **缺失与 0 必须可区分**：没有测到的字段不要写 key，不要用 `0` 代替；`0` 表示真实测得 0。
- **总输入必须是总数**：若某一侧只能给出"不含命中部分"的输入数，需要先加上命中部分再上报，
  否则命中率的分母偏小。
- 缺少总输入数时（既没有 `prompt_tokens` 也没有 `input_tokens`），该请求按"没有 usage"处理，
  走 `estimated` 估算口径，不会产出看起来真实的命中率。
- 时延字段缺失时，输出中对应字段整项缺省。

## 响应

| 响应状态 | 条件 | Body |
|---|---|---|
| `202 Accepted` | 成功写入一条完整记录 | 空 |
| `400 Bad Request` | body 非合法 JSON object、缺少 endpoint key，或 metadata header 非有效非空文本 | 错误说明文本 |
| `405 Method Not Allowed` | 非 `POST /requests` | 框架默认响应 |
| `413 Payload Too Large` | body 超过 `8 MiB` | 框架默认响应 |
| `500 Internal Server Error` | 记录无法安全持久化 | `failed to persist captured request` |
