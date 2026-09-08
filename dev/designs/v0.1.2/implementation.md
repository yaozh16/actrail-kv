<!-- 本文件记录 v0.1.2 的实现边界、设计索引与验收结果。 -->
# v0.1.2 实现设计

## 需求

- [开发目标](../../requirements/v0.1.2/target.md)

## 报告路径

报告模块使用统一的父目录解析函数，未取得父路径或父路径为空时归一为 `.`。路径冲突校验与同目录原子写入共用该函数。端到端脚本在临时目录的子进程中验证裸文件名创建、替换和输入保护。

## 部署脚本

`docs/deployment/fullchain-deploy-linux.sh` 是 Linux 管理入口，通过绝对脚本目录加载 `docs/deployment/fullchain-deploy-linux/report.sh` 中的报告命令。Session 数量取 `session_analysis.timelines` 的数组长度。部署分发需保留入口与子目录的相对布局。

## 发布与文档

workspace 包版本为 v0.1.2，分析产物使用 schema v0.1.1。上游插件执行 Anthropic → OAI 转换，分析器消费转换后的请求。版本需求与实现设计存放在 `dev/`，当前架构和操作说明存放在 `docs/`。

## 部署环境与中性命名

- 显式 `KV_ROOT` 优先；未设置时根据脚本目录定位部署根目录，定位失败直接报错。`DOC_ROOT` 默认为 `$KV_ROOT/doc`，与 Windows 一致，仍可显式覆盖。
- `RUSTUP_HOME`、`CARGO_HOME` 继承调用环境。显式 `CARGO_BIN` 优先加入 PATH；否则优先使用 PATH 中的 Cargo，仅在未找到时查找已有 `CARGO_HOME/bin` 或当前用户的 `.cargo/bin`。对应 cargo 可执行时补充 PATH。
- 集成需求文件使用 `feature-integration.md` 和“v0.1.1 特性集成要求”。
- `DOC_ROOT` 指定报告服务目录；工具链路径通过部署环境配置。

## 配置契约

候选发现由 `template_compatibility_threshold` 和 `max_dynamic_coverage_ratio` 控制。

`text_similarity_threshold` 是已废弃的兼容字段，支持配置读写、有限值区间校验及结果快照。修改该值只改变快照，诊断结果相同。

## 已失效决策索引

- 将 `text_similarity_threshold` 描述为候选文本相似度门槛的说明已失效；当前含义为兼容字段，见[配置参考](../../../docs/configuration.md)和[分析输出](../../../docs/architectures/analyze/output.md)。[v0.1.0 配置设计](../v0.1.0/config-file.md#兼容性)保留历史序列化记录并标注失效用途。
- 部署脚本使用固定个人目录及临时 Cargo 缓存目录的默认策略已失效，当前规则见[部署环境与中性命名](#部署环境与中性命名)。

## Windows 兼容集成

需求见 [Windows 兼容集成](../../requirements/v0.1.2/windows-integration.md)。Windows 追加文件通过 `FILE_SHARE_READ` 支持单写者和并发读者，Unix 使用 advisory 文件锁。语料读取接口返回 `Result`，I/O 失败返回错误及行号，坏 JSON 行记为 skipped。Session 标识使用 `session_id`。Windows 部署入口兼容 PowerShell 5.1。

## 验收结果

2026-09-08，在 Linux、Rust 1.90.0 环境完成：

| 验收 | 结果 |
|---|---|
| `cargo test --workspace --locked` | 140 个测试通过，包含读取失败中止与写入期间读取回归。 |
| `cargo test --release --workspace --locked` | 140 个测试通过。 |
| `cargo fmt --all -- --check` | 通过。 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 通过。 |
| `cargo check --workspace --all-targets --locked --target x86_64-pc-windows-gnu` | 全 workspace 及测试目标交叉编译检查通过。 |
| `bash tests/end_to_end/three_binaries.sh` | 三程序链路、确定性、XSS、Session、裸文件名创建/替换及输入保护通过；旧文本阈值变化只影响快照。 |
| Linux 脚本 | 语法、入口帮助、含空格路径和外部工作目录调用通过；Session 样例显示 `sessions 1` 并生成 HTML。 |
| 部署环境 | PATH 优先、显式 CARGO_BIN、自定义 CARGO_HOME 回退及当前用户标准安装目录回退四组通过；环境变量继承、默认目录和显式覆盖通过。 |
| 文件与文档 | Linux 入口 473 行，报告模块 46 行；代码文件长度、职责注释、文档链接与目录索引检查通过，未发现用户批注。 |
| 原生 Windows 文件共享与 PowerShell 5.1 运行 | 未执行。 |
