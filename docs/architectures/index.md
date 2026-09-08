<!-- 本文件是当前架构文档的入口索引。 -->
# Architecture

## 二进制契约

| 二进制 | 输入 | 输出 | 专属文档 |
|---|---|---|---|
| `actrail-kv-receiver` | 完整 LLM HTTP JSON request | `requests.ndjson` | [输入](receiver/input.md) / [输出](receiver/output.md) |
| `actrail-kv-analyze` | `requests.ndjson` | `analysis.json` | [输入](analyze/input.md) / [流水线](analyze/pipeline.md) / [输出](analyze/output.md) |
| `actrail-kv-report` | `analysis.json` | `report.html` | [输入](report/input.md) / [输出](report/output.md) |

## 横切架构

- [部署交互视图](deployment.md)：外部采集链路、三个二进制及报告产物流向。
- [全链路部署指南](../deployment/fullchain-deployment-guide.md)：上游接入、协议转换与部署脚本操作。
- [代码布局](code-layout.md)：源码模块和三二进制调用关系。
- [运行约束](constraints.md)：原子写入、数据处理与部署边界。
- [文档布局](doc-layout.md)：当前文档树及文档落点规则。

## 分析边界

分析过程不需要访问外部网络、数据库、模型服务或 tokenizer。我们会根据收到的真实请求，指出哪些上下文结构可能破坏缓存，以及可以怎样调整；但在拿不到缓存命中数据时，不会把这些线索说成已经发生的 KV miss，也不会凭空估算 Token 或金额收益。
