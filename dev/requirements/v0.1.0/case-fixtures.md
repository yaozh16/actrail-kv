<!-- 本文件记录真实语料回归 fixture 需求。 -->
# 真实语料回归 Fixture 需求

## 背景

新增能力缺少“真实采集语料”级别的回归保护，人工验证物
（workspace/policy/json/insert/natural/session/context-refresh）没有入库。

## 目标

把真实 xiaoO 语料整理到 `examples/cases/`，并用集成测试锁定默认阈值下的检出行为。

## 验收

- workspace：检出 `content_variation`；
- policy：检出 `fixed_variants`；
- json：检出 content 类缺陷（等价事实由带标签单测锁定）；
- insert：至少检出 1 个缺陷；
- natural：可正常分析；
- session：`session_reports` 非空且事件含 lcp 证据。
- session-context-refresh：刷新 session 只产生 1 次 warning，append-only session 为 0。
