<!-- 本文件索引 0.2.0 的有效设计决策及被扩展的历史约定。 -->
# 0.2.0 设计决策

| 编号 | 决策 | 依据与生效范围 |
| --- | --- | --- |
| D01 | C ABI 动态库加 Rust 安全 SDK | 宿主保持编译产物不变，动态库可独立升级，后续 C++ 可复用 ABI |
| D02 | UTF-8 JSON 缓冲区加指针和长度 | 复用现有数据契约，Rust SDK 提供类型化封装 |
| D03 | receiver、analyzer、reporter 独立 `.so` | 生命周期与触发时机独立，分别加载和升级 |
| D04 | HTTP 与 SDK 共用公共采集服务 | 统一业务校验、采集时间和写入成功语义 |
| D05 | 升级通过停止宿主、替换库、重启生效 | 实例和内存只归属创建它们的库版本 |
| D06 | 首版采用阻塞 ABI 和同实例串行调用 | 简化外部生命周期契约，异步运行时封装于库内 |
| D07 | 配置在 JSON 信封内整体传递 TOML | 复用现有配置解析，默认值由动态库内所属模块维护 |
| D08 | receiver 持续持久化，analyzer 按需筛选读取 | 常态采集与按需分析通过采集存储解耦 |
| D09 | SDK 分析结果直接传给 reporter，产物显式导出 | `render` 借用完整分析响应及独立参数缓冲区，CLI 保持文件交接行为 |

## 0.2.0 契约修订索引

此前将 `submit`、`analyze` 和 `render` 统一描述为单请求缓冲区的接口形状已失效。D09 将 `render` 改为分析响应与渲染参数两个输入缓冲区，并为 analyzer 与 reporter 增加独立 `export` 入口。有效接口见 [C ABI 与兼容契约](../../../docs/architectures/sdk/abi.md)。

## 历史约定替代索引

| 历史文档 | 变更范围 | 0.2.0 生效设计 |
| --- | --- | --- |
| [0.1.0 Receiver 接入](../v0.1.0/receiver-integration/target.md) | HTTP 作为唯一接入形态的约定被双通道采集取代，原 HTTP 协议继续有效 | [公共采集服务](../../../docs/architectures/receiver/index.md) |
| [0.1.0 workspace 布局](../../designs/v0.1.0/workspace-layout.md) | 库统一直接置于 `crates/<crate>` 的描述扩展为 SDK 分层目录 | [SDK 交付结构](../../../docs/architectures/sdk/index.md) |

0.2.0 尚未实现的结构在代码布局中标记为待开始。现有部署与运行约束继续适用于现有 CLI，新增行为以版本设计标明。
