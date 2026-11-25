<!-- 本文件描述 actrail-kv 各命令的部署边界和数据流。 -->
# 部署架构

供给方按 [Receiver HTTP 协议](../reference/receiver/input.md)上报完整模型请求。[部署指南](../deployment/fullchain-deployment-guide.md)提供构建与接收验证步骤。

```mermaid
flowchart LR
    Source[数据供给方] -->|POST /requests| Receiver
    Receiver --> Store[采集存储]
    Store --> Analyze --> Analysis[analysis.json] --> Report
    Store --> Export --> NDJSON[导出 NDJSON]
```

采用托管存储时，Receiver、Analyzer 与 Export 通过同一套[存储配置](storage/index.md)访问 SQLite 或分片 NDJSON。接收可以持续运行，分析、导出和生成报告可以按需或定时执行。
