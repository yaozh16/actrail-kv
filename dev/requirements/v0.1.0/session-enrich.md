<!-- 本文件记录会话 prefix-switch 证据增强需求。 -->
# 会话 Prefix-Switch 证据增强需求

## 背景

会话相邻对已有 lcp/recomputed/stable 字节，但缺少“前缀收缩被放大”“稳定块以 A4B2C2 形态呈现”“平均收缩量”等直观证据，AAAABBCC 类问题不易一眼识别。

## 目标

- 每个会话事件增加 `previous_pair_lcp_bytes` 与 `prefix_cut_bytes`（前缀收缩）。
- 每个事件输出下一条请求内容性单位的稳定块 run-length 摘要（如 `abc12345x4/…`）。
- 会话报告增加 `total_prefix_cut_bytes` 与 `avg_prefix_cut_bytes`。
- report.html 证据表展示上述字段。

## 验收

- 相邻对 LCP 出现收缩时 `prefix_cut_bytes > 0`；首事件为 0。
- 重复相同内容块（x4）在 run-length 摘要中可见。
- 旧 analysis.json（无这些字段）仍可解析。
