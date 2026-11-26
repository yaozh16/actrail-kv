<!-- 本文件索引迁移后的 Rust workspace 布局。 -->
# Rust workspace 布局

> 0.2.0 变更：库目录统一为 `crates/<crate>` 的描述扩展为 SDK 分层目录。见[SDK 交付结构](../../../docs/architectures/sdk/index.md)及[历史约定替代索引](../../requirements/v0.2.0/decisions.md)。

- [源码与命令目录](../../../docs/architectures/code-layout.md)
- [目录迁移验收范围](../../requirements/v0.1.0/workspace-layout/target.md)

库路径采用 `crates/<crate>`，命令路径采用 `crates/cmd/<command>`。此布局取代原有的根级 crate 目录。测试源码归档在根目录 `tests/`，集成测试由 package 清单声明，单元测试通过受 `cfg(test)` 控制的模块入口加载。此约定取代 crate 内的测试源码目录。跨命令测试和公共示例以仓库根目录为相对路径起点。
