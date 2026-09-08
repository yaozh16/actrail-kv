<!-- 本文件记录 v0.1.2 的实现边界、设计索引与验收结果。 -->
# v0.1.2 实现设计

## 需求

- [开发目标](../../requirements/v0.1.2/target.md)

## 报告路径

报告模块使用统一的父目录解析函数，未取得父路径或父路径为空时归一为 `.`。路径冲突校验与同目录原子写入共用该函数。端到端脚本在临时目录以裸文件名运行 report，验证创建与替换行为，避免测试修改进程全局工作目录。

## 部署脚本

保留 `docs/deployment/fullchain-deploy-linux.sh` 入口，将 `cmd_report` 按完整职责提取到 `docs/deployment/fullchain-deploy-linux/report.sh`，通过入口的绝对脚本目录加载。仅将 Session 统计改为 `session_analysis.timelines`，其余命令及报告权限保持原行为。部署分发需保留入口与子目录的相对布局。

## 发布与文档

workspace 发布版本更新为 v0.1.2；包版本与分析产物 schema 独立管理。输入说明明确上游插件执行 Anthropic → OAI 转换，分析器消费转换后的请求。更新 v0.1.1 状态与当前文档布局，保留原版本设计记录。

## 部署环境与中性命名

- 显式 `KV_ROOT` 优先；未设置时根据脚本目录定位部署根目录，定位失败直接报错。`DOC_ROOT` 默认为 `$KV_ROOT/doc`，与 Windows 一致，仍可显式覆盖。
- 不设置或覆盖 `RUSTUP_HOME`、`CARGO_HOME`。显式 `CARGO_BIN` 优先加入 PATH；否则优先使用 PATH 中的 Cargo，仅在未找到时查找已有 `CARGO_HOME/bin` 或当前用户的 `.cargo/bin`。仅在对应 cargo 可执行时补充 PATH。
- 集成需求文件使用 `feature-integration.md` 和“v0.1.1 特性集成要求”；同步目录索引与引用，保留技术事实及 Git 历史。
- 旧报告服务如依赖固定目录，应显式设置 `DOC_ROOT`；跨用户工具链路径由部署环境显式提供。

## 配置契约

`text_similarity_threshold` 是旧局部差异分类的遗留参数：提交 `dfa12c9` 将其用于 `local_text_similarity`，统一 P1/X/P2 重写提交 `3a06cdf` 删除了对应诊断用途。当前候选发现通过 `template_compatibility_threshold` 和 `max_dynamic_coverage_ratio` 判定；该遗留字段不应作为新的候选门槛接入。

保留该字段的配置读写、有限值区间校验及结果快照，明确标注“已废弃，仅为兼容保留，不参与分析”。保持 schema v0.1.1；通过端到端配置变更验证仅快照字段变化，分析结果保持一致。

## 已失效决策索引

- 将 `text_similarity_threshold` 描述为候选文本相似度门槛的说明已失效；当前含义为兼容字段，见[配置参考](../../../docs/configuration.md)和[分析输出](../../../docs/architectures/analyze/output.md)。[v0.1.0 配置设计](../v0.1.0/config-file.md#兼容性)保留历史序列化记录并标注失效用途。
- 部署脚本使用固定个人目录及临时 Cargo 缓存目录的默认策略已失效，当前规则见[部署环境与中性命名](#部署环境与中性命名)。

## 验收结果

2026-09-08，在 Linux、Rust 1.90.0 环境完成：

| 验收 | 结果 |
|---|---|
| `cargo test --workspace --locked` | 138 个测试通过。 |
| `cargo test --release --workspace --locked` | 138 个测试通过。 |
| `cargo fmt --all -- --check` | 通过。 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 通过。 |
| `bash tests/end_to_end/three_binaries.sh` | 三程序链路、确定性、XSS、Session、裸文件名创建/替换及输入保护通过；旧文本阈值变化只影响快照。 |
| Linux 脚本 | `bash -n` 与入口帮助通过；在含空格的临时目录，从任意工作目录调用完整 report 入口，以本次构建的 debug 二进制运行 Session 样例，正确显示 `sessions 1` 并生成 HTML。 |
| 拆分等价性 | 完整 `cmd_report` 函数提取为子模块，函数内仅修改 Session 字段；环境路径配置单独按 13 更新，停止、权限及其他命令函数内容保持一致。 |
| 部署环境 | 以完整 report 入口分别验证 PATH 优先、显式 CARGO_BIN、自定义 CARGO_HOME 回退及当前用户标准安装目录回退；调用真实 debug 二进制生成 Session 报告，四组均通过。显式 RUSTUP_HOME/CARGO_HOME 保持原值，未设置时保持未设置。默认部署根目录、doc 报告目录及显式覆盖通过。 |
| 文件与文档 | Linux 入口 473 行，报告模块 46 行；全仓代码文件不超过 500 行且有开头注释；文档本地链接有效，集成需求改为中性名称并同步引用，未发现用户批注。 |

本次未在 Windows 环境执行验证；Windows 脚本未修改。包发布版本为 v0.1.2，实际生成的分析产物继续标记 schema v0.1.1。

13 仅变更 Shell 与文档，采用脚本语法、环境矩阵和实际报告入口验证；Rust 全量测试结果沿用上表，Rust 代码自该验收后未变化。
