<!-- 本文件定义会话 prefix-switch 诊断的模块、数据流与证据契约。 -->
# 会话内 Prefix-Switch 设计

## 状态

- 本方案已失效，由 v0.1.1 的 `session_id + session_analysis` 线性时间线设计取代。
- 当前设计见 `dev/designs/v0.1.1/session-prefix-and-multi-site.md`。

## 数据流

```text
CapturedRequest(+session_key)
  → CorpusRecord.session_key
  → CacheSequence（既有投影）
  → session::analyze（按 session_key + 文件顺序分组）
  → SessionSwitchEvent[] → SessionReport[]
  → AnalysisResult.session_reports
  → report.html 证据表
```

`session_key` 是 envelope 顶层可选字段；不进入 `comparison`，不参与 comparison group，避免拆散模板分析。

## 相邻对判定

对同一 session 的相邻两条请求（文件顺序即接收顺序）：

1. 计算公共前缀单位数与字节数（单位内容/层级相同才算公共）。
2. 分类：
   - `append`：上一条是下一条前缀（可含完全相同）。
   - `reorder`：两个单位多重集完全相同、元素在各自请求内唯一、但顺序不同。
   - `reset`：公共前缀为 0。
   - `fork`：其余情况（有公共前缀但下一条非 append 且非 reorder）。
3. 字节证据：
   - `recomputed_bytes = next_total_bytes - lcp_bytes`；
   - `stable_after_switch_bytes`：第一个分歧点之后、两条请求中按原顺序连续对齐的最长相同单位段的字节数（CC 证据；没有则为 0）。

## 产物

`AnalysisResult` 增加可选字段 `session_reports: Vec<SessionReport>`（默认空，旧文件兼容）。每个报告含 session_key、endpoint/model、请求数、四类事件计数、浪费字节合计与事件明细（prev/next request id、类型、lcp、重算字节、可恢复稳定字节）。

## 边界

- 无 session_key 的记录不进入会话诊断。
- 不做业务合理性判断，不生成“是否可避免”的建议。
- 排序稳定：文件顺序决定时间序；报告按浪费降序、session_key 升序打平。
