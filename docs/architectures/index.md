<!-- 本文件是当前架构文档的入口索引。 -->
# Architecture

## 二进制契约

| 二进制 | 输入 | 输出 | 专属文档 |
|---|---|---|---|
| `actrail-kv-receiver` | 完整 LLM HTTP JSON request | `requests.ndjson` | [输入](receiver/input.md) / [输出](receiver/output.md) |
| `actrail-kv-analyze` | `requests.ndjson` | `analysis.json` | [输入](analyze/input.md) / [流水线](analyze/pipeline.md) / [输出](analyze/output.md) |
| `actrail-kv-report` | `analysis.json` | `report.html` | [输入](report/input.md) / [输出](report/output.md) |

## 横切架构

- [代码布局](code-layout.md)：源码模块和三二进制调用关系。
- [运行约束](constraints.md)：原子写入、数据处理与部署边界。
- [文档布局](doc-layout.md)：当前文档树及文档落点规则。

运行时不访问外部网络、数据库、模型服务或 tokenizer。结论只描述可观察请求结构的风险和优化机会，不声称真实 KV miss、Token 或金额收益。
