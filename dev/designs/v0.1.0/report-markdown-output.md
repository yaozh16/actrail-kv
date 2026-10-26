<!-- 本文件定义 HTML/Markdown 双报告的输出编排、Markdown 模块与测试边界。 -->
# HTML 与 Markdown 双报告设计

后续 HTML renderer 已按[编译期模板设计](report-html-template.md)重构；本设计中的
“HTML 渲染函数保持不变”只描述双报告首次落地时的迁移约束，当前 HTML 输出契约仍有效。

## 数据流

```text
analysis.json
  → load_result（只执行一次完整校验）
  → AnalysisResult
      ├─ html_report_renderer（视图模型 + 编译期模板）
      └─ markdown_report_renderer（新增 GFM）
  → 按 --format 选择产物
  → 原子写入 report.html / report.md
```

Markdown 不从 HTML 字符串反解析，避免 HTML 转文本时丢失表格、锚点和变体语义。

## 输出编排

在 reporter library 中定义：

```text
ReportFormat { Html, Markdown }
ReportSelection { Both, One(ReportFormat) }
```

CLI 的可选 `--format` 映射为 `ReportSelection`。`write_report` 增加可接收选择参数的入口，
同时保留只写 HTML 的兼容函数供现有调用和测试使用。

路径规则：

- `Both`：去掉末尾 `.html` 或 `.md` 后作为 base，分别追加 `.html`、`.md`；
- `One(Html)` / `One(Markdown)`：严格使用 `--output`；
- 所有路径在渲染前完成输入/输出碰撞校验。

两份报告先全部渲染到内存，再创建同目录临时文件。默认双报告模式只有在两份内容都成功渲染后才开始发布。
最终分别使用原子 rename 发布；单文件仍保持现有原子写语义。

## Markdown renderer

新增目录：

```text
reporter/src/markdown_report_renderer/
├── mod.rs          # render_markdown 与总章节编排
├── format.rs       # GFM 转义、短 ID、百分比、字节和表格辅助
├── prefix_reuse.rs # 结构复用率与结构/真实命中对照
├── defects.rs      # Top K 与缺陷详情
├── session.rs      # 会话上下文刷新
├── cache.rs        # KV 缓存指标
└── prefix_tree.rs  # 模板前缀树
```

HTML renderer 只允许进行公共编排层所需的机械改动，不改变任何现有渲染函数的内容。

## GFM 映射

| HTML 内容 | Markdown 内容 |
|---|---|
| 标题与分层 | `#` / `##` / `###` |
| 数据表格 | GFM pipe table |
| 复用率色条 | 当前值、上界、目标、三段字节的表格与结论句 |
| 逐轮 spark | 每轮前缀复用度列表 |
| 成员衰减 SVG | 每轮片段序号、存活数、支持率的表 |
| 折叠事件 | 独立子标题与事件表 |
| 缺陷锚点 | 唯一缺陷标题及 GFM 标题链接 |
| CSS / 布局 | 不输出 |

所有文本进入 Markdown 行内代码或经过统一转义；不使用 raw HTML。表格单元格至少处理：

- `|` → `\|`
- 换行 → `<br>` 不符合纯 GFM，因此统一压成空格或分段文本
- 反引号 → 行内代码改用安全围栏长度或普通转义
- `#`、列表标记和链接标记只在行首或链接语法位置转义

## 测试

1. 同一 fixture 的 HTML 渲染结果保持不变。
2. 默认模式生成两份文件；只选格式时只生成一份。
3. `report.html`、`report.md`、无后缀三种输出名派生正确。
4. Markdown 包含 HTML 的每个报告区块及关键数字/空态。
5. Markdown 不包含 `<style>`、`<script>`、`<svg>` 和 `<details>`。
6. Markdown 表格中的特殊字符保持结构稳定。
7. 输入输出同路径仍被拒绝。
8. 端到端脚本默认生成两种格式并分别校验内容。

## 文档维护

实现时同步更新：

- `README.md`；
- `docs/configuration.md`；
- `docs/architectures/index.md`；
- `docs/architectures/code-layout.md`；
- `docs/architectures/report/output.md`；
- 部署脚本及全链路部署指南。
