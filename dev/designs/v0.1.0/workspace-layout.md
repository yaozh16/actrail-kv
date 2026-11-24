<!-- 本文件索引迁移后的 Rust workspace 布局。 -->
# Rust workspace 布局

- [源码与命令目录](../../../docs/architectures/code-layout.md)
- [目录迁移验收范围](../../requirements/v0.1.0/workspace-layout/target.md)

库路径采用 `crates/<crate>`，命令路径采用 `crates/cmd/<command>`。此布局取代原有的根级 crate 目录。跨命令测试和公共示例以仓库根目录为相对路径起点。
