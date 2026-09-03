<!-- 本文件定义会话证据增强的实现边界。 -->
# 会话 Prefix-Switch 证据增强设计

## 字段

`SessionSwitchEvent` 增加：

- `previous_pair_lcp_bytes`：相邻前一对公共前缀字节（首事件 0）；
- `prefix_cut_bytes = previous_pair_lcp.saturating_sub(lcp_bytes)`；
- `next_block_runs`：下一条内容性单位连续相同块的 run-length 摘要。

`SessionReport` 增加 `total_prefix_cut_bytes` 与 `avg_prefix_cut_bytes`（整数平均）。

## 实现

- `lineage.rs`：循环中维护 `previous_lcp_bytes`；`run_length_summary` 只统计内容性单位（VisibleText/Tool*），用内容 SHA-256 前 4 字节做块身份，连续相同块计一次 run。
- reporter：事件表新增“前缀收缩”“块 run”列，报告头增加收缩总量与平均。

## 边界

- role/结构噪声不参与块 run-length；
- run-length 是机器可读证据摘要，不做业务解释。
