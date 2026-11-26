<!-- 本文件索引 0.2.0 的动态库、Rust SDK 和 Receiver 通道拆分设计。 -->
# 0.2.0 设计索引

状态：需求与设计已记录，业务实现待开始。

## 设计文档

- [版本需求与验收](../../requirements/v0.2.0/target.md)
- [决策与历史约定替代索引](../../requirements/v0.2.0/decisions.md)
- [原生动态库与 Rust SDK](../../../docs/architectures/sdk/index.md)
- [C ABI 与兼容契约](../../../docs/architectures/sdk/abi.md)
- [Receiver 采集服务与通道](../../../docs/architectures/receiver/index.md)
- [代码布局](../../../docs/architectures/code-layout.md)

## 实施顺序

1. 定义 receiver 的公共契约和工厂，抽取业务处理至公共采集服务，保持 HTTP 协议回归通过。
2. 定义 ABI 基础类型、符号、状态码和 JSON schema，建立头文件与 ABI 契约验证。
3. 实现 receiver 动态库和 Rust SDK 的直接提交、运行时管理及关闭流程。
4. 分离分析计算与文件发布，提供内存 `AnalysisOutput` 和 `ReportOutput`，实现 reporter 直接借用分析结果及独立显式导出，消除 SDK 路径对全局 locale 的依赖。
5. 用已编译宿主验证兼容替换、版本拒绝、结果缓冲区生命周期、无文件中转渲染以及 HTTP 与 SDK 的语义一致性，回归 CLI 文件生成和读取行为。
6. 交付构建和加载说明、Rust 示例、C 头文件及更新后的接口参考。

## 实现约束

新增代码按能力分层，文件超过 500 行时按职责拆分。每个代码文件开头说明职责。公共构造入口使用 Factory，配置整体透传。只在 ABI 与加载边界引入必要的 unsafe，复用现有存储、分析、渲染和配置框架。
