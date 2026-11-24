<!-- 本文件记录 Rust workspace 的目录迁移范围与验收标准。 -->
# Rust workspace 目录迁移

- 五个库 crate 放在 `crates/`，四个命令 crate 放在 `crates/cmd/`。
- 仓库根目录提供 Cargo workspace 入口、示例语料、跨命令端到端测试和文档。
- 更新 workspace 成员、本地依赖、测试语料和文档引用。构建、分析语料回归和命令行流程应从仓库根目录按原方式运行。
