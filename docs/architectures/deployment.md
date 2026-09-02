<!-- 本文件描述 actrail-kv 与外部采集组件的部署和数据交互关系。 -->
# 部署交互视图

`actrail-kv` 不限定请求必须由哪一种组件采集。上游只要能够导出完整的 LLM 请求，并转换成 Receiver 接受的 HTTP JSON，就可以接入。下图同时展示了 Actrail 插件链路和网关、Hook 等通用链路；它们是可选入口，不要求同时部署。

![actrail-kv 部署交互视图](assets/deployment-view.png)

图中的 `requests.ndjson` 和 `analysis.json` 是三个二进制之间的显式文件边界。因此，接收可以持续运行，分析和报告生成则可以按需或定时离线执行。

可编辑源文件：[deployment-view.puml](assets/deployment-view.puml)。
