<!-- 本文件是当前架构文档的入口索引。 -->
# Architecture

## 二进制契约

| 二进制 | 输入 | 输出 | 专属文档 |
|---|---|---|---|
| `actrail-kv-receiver` | 完整 LLM HTTP JSON request | SQLite 或 NDJSON 采集存储 | [输入](receiver/input.md) / [输出](receiver/output.md) |
| `actrail-kv-analyze` | 采集存储或 NDJSON 文件 | `analysis.json` | [输入](analyze/input.md) / [流水线](analyze/pipeline.md) / [输出](analyze/output.md) |
| `actrail-kv-export` | SQLite 或托管 NDJSON 采集存储 | NDJSON 文件 | [采集存储](storage/index.md) |
| `actrail-kv-report` | `analysis.json` | `report.html` / `report.md` | [输入](report/input.md) / [输出](report/output.md) |

## 横切架构

- [部署交互视图](deployment.md)展示外部采集链路及单文件报告流程。
- [代码布局](code-layout.md)：源码模块和命令调用关系。
- [采集存储](storage/index.md)：共享接口、后端与保留期。
- [运行约束](constraints.md)：原子写入、数据处理与部署边界。
- [文档布局](doc-layout.md)：当前文档树及文档落点规则。

## 分析边界

分析读取配置指定的本地语料。算法可离线运行，并根据真实请求指出可能影响缓存的上下文结构与改进方式。缓存命中数据缺失时，报告展示结构证据及其推断范围。
