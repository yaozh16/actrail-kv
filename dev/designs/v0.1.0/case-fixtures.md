<!-- 本文件定义 fixture 的组织与测试口径。 -->
# 真实语料 Fixture 设计

## 组织

`examples/cases/` 下每个文件是 receiver 输出格式的 NDJSON：

- `workspace/policy/json/insert/natural.ndjson`：真实 xiaoO 请求；
- `session.ndjson`：带 `X-Actrail-Session-Key` 注入的同批请求。

## 测试

`analyzer/tests/case_fixtures.rs` 用放开对齐预算的 `AnalysisOptions` 读取 fixture，按 fixture 断言模板缺陷事实或会话证据存在；语料同时作为 examples 供手动复现。
