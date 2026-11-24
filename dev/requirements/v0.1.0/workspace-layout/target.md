<!-- 本文件记录 Rust workspace 的目录迁移范围与验收标准。 -->
# Rust workspace 目录迁移

- 五个库 crate 放在 `crates/`，四个命令 crate 放在 `crates/cmd/`。
- 仓库根目录提供 Cargo workspace 入口、示例语料和文档。独立测试及单元测试源码统一归档到根目录的 `tests/`，按测试范围和所属 crate 组织。
- 更新 workspace 成员、本地依赖、测试语料和文档引用。构建、分析语料回归和命令行流程应从仓库根目录按原方式运行。
- Cargo 按原有 package 运行其单元测试和集成测试。单元测试通过测试模块入口访问原模块内部行为。
