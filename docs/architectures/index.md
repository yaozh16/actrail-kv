<!-- 本文件是当前架构文档的入口索引。 -->
# Architecture

各命令的输入协议和产物格式见[接口参考](../reference/index.md)。

## 横切架构

- [部署架构](deployment.md)展示各命令之间的数据流。
- [分析流水线](analyze/pipeline.md)展示从语料分组到 Top K 的过程。
- [代码布局](code-layout.md)：源码模块和命令调用关系。
- [采集存储](storage/index.md)：共享接口、后端与保留期。
- [运行约束](constraints.md)：原子写入、数据处理与部署边界。
- [文档布局](doc-layout.md)：当前文档树及文档落点规则。

## 分析边界

分析读取配置指定的本地语料。算法可离线运行，并根据真实请求指出可能影响缓存的上下文结构与改进方式。缓存命中数据缺失时，报告展示结构证据及其推断范围。

## 0.2.0 待实现设计

- [原生动态库与 Rust SDK](sdk/index.md)：C ABI 交付、动态加载与独立升级。
- [C ABI 与兼容契约](sdk/abi.md)：JSON 缓冲区、内存所有权与实例生命周期。
- [Receiver 采集服务与通道](receiver/index.md)：HTTP 与函数提交共享业务逻辑。
