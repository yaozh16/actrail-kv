<!-- 本文件描述 actrail-kv 与外部采集组件的部署和数据交互关系。 -->
# 部署交互视图

`actrail-kv` 不限定请求必须由哪一种组件采集。上游只要能够导出完整的 LLM 请求，并转换成 Receiver 接受的 HTTP JSON，就可以接入。下图同时展示了 Actrail 插件链路和网关、Hook 等通用链路；它们是可选入口，不要求同时部署。

![actrail-kv 部署交互视图](assets/deployment-view.png)

图展示单文件 NDJSON 部署。采用托管存储时，Receiver 与 Analyzer 通过同一套[存储配置](storage/index.md)访问 SQLite 或分片 NDJSON。Export 也可读取该存储。接收可以持续运行，分析和报告生成可以按需或定时执行。

可编辑源文件：[deployment-view.puml](assets/deployment-view.puml)。
