<!-- 本文件记录 v0.1.0 部署交互视图的交付要求。 -->
# Deployment Interaction View

- 用一张带外部环境的 Container 视图描述完整请求如何进入 `actrail-kv`。
- 展示两条可选接入链路：`Agent → Actrail → Actrail Plugin` 与 `Agent → 网关或 Hook`。
- 在 `actrail-kv` 边界内展示 Receiver、Analyze、Report 三个独立二进制及其文件交接。
- 明确最终产物为可供优化人员阅读的 `report.html`。
- 视图不得暗示 Actrail 是唯一数据源，也不得把采集、格式转换描述成分析器的核心能力。
