<!-- 本文件记录每请求 KV 命中率指标需求。 -->
# 每请求 KV 命中率需求

## 背景

结构诊断只能给出"哪些内容阻断了可复用前缀"的结构代理，无法回答上游实际命中了多少。
采集侧（例如 agent-cassette）能看到上游响应的 usage，把它随请求一起上报后，
`actrail-kv` 就能给出逐请求的真实命中情况，并与结构代理互相印证。

## 目标

1. 直推 payload 允许携带保留字段 `_actrail_response_usage`
   （`prompt_tokens` / `cached_tokens` / `completion_tokens`）；receiver 不改写 payload。
2. analyzer 输出 `cache_metrics[]`，按会话分组、组内按 `captured_at` 与输入行序稳定排序：
   逐请求 `prompt_tokens` / `cached_tokens` / `miss_tokens` / `output_tokens`、
   `hit_rate = cached / prompt`、与同一会话上一条的 `hit_rate_delta`、以及 `payload_bytes`。
3. 上游没有返回 usage 时按与上一条请求 payload 的公共前缀估算：token 字段全部缺省，
   `hit_rate` 为字节比代理值，`basis = estimated`；有 usage 时为 `reported`。
4. report 渲染「KV 缓存命中率」区块，按 `basis` 标注"真实"或"估算"，不得混列。

## 验收

- 含 `_actrail_response_usage` 的连续请求输出 `basis = reported` 且给出非空环比变化；
- 不含该字段时全部为 `estimated`，token 字段缺省，`estimated_lcp_bytes` 有值；
- 旧版 `analysis.json`（无 `cache_metrics`）仍可解析，报告不渲染该区块；
- 报告中 `estimated` 的命中率不会被展示为真实命中率。
