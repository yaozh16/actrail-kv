<!-- 本文件定义 HTML 报告的编译期模板、展示模型与模块拆分设计。 -->
# HTML 报告编译期模板设计

## 数据流

```text
analysis.json
  → load_result
  → AnalysisResult
  → HtmlReportView
      ├─ 本地化 common 文案
      ├─ Top K / 缺陷 / 复用率 / 缓存 view
      ├─ 会话证据 view
      └─ 前缀树 view
  → Askama 编译期模板
  → report.html
```

Markdown 保持 `AnalysisResult → markdown_report_renderer → report.md` 的独立数据流。

## 技术选择

模板使用 Askama。模板在编译期解析和检查，并生成包含渲染逻辑的 Rust 代码；
release 二进制不依赖运行时模板目录。

不使用单个超长 `template.html`，也不自行实现占位符替换：

- `layout.html` 保存文档外壳和区块编排；
- `styles.css` 保存全局静态样式；
- `sections/` 保存各报告区块；
- Rust 中的 view builder 负责从分析结果构造展示数据。

## 代码布局

```text
reporter/
├── templates/report/
│   ├── layout.html
│   ├── styles.css
│   └── sections/
│       ├── prefix-reuse.html
│       ├── top-k.html
│       ├── defect.html
│       ├── session.html
│       ├── cache.html
│       └── prefix-tree.html
└── src/html_report_renderer/
    ├── mod.rs
    ├── template.rs
    └── view/
        ├── mod.rs
        ├── defects.rs
        ├── prefix_reuse.rs
        ├── session.rs
        ├── cache.rs
        └── prefix_tree.rs
```

## View Model 边界

`HtmlReportView` 拥有最终模板所需数据：

- `lang`、标题、摘要和限制说明；
- 各区块是否出现的 `Option`；
- 已排序并已本地化的行数据；
- 已格式化的百分比、字节、短 ID 和状态标签；
- 供可信模板使用的受控枚举状态。

模板不调用 locale 查询、不处理 `anyhow::Result`、不执行聚合或重新排序。
普通文本不预先 HTML 转义，由 Askama 自动转义；仅编译期静态 partial 可以原样嵌入。

## 异常输入防护

`prefix_view` 是由 `sequence_child` 连接的有向链。加载 `analysis.json` 时必须校验
`node_count`、节点 ID 唯一性、子节点引用存在性，并从 `seq-0` 验证整条链可达且无环。
校验失败时直接拒绝输入，不进入模板渲染。

模板渲染器仍保留已访问节点检测作为第二道防护，保证直接调用公开渲染 API 时也不会因
环路无限循环或持续分配内存。

## 迁移与兼容

先固定现有输出回归，再迁移页面外壳，随后迁移低风险区块，最后迁移前缀树。
HTML 输出以页面文本和 DOM 结构等价为验收口径，允许模板缩进产生空白差异。
现有公开入口 `render_html` 与 `render_html_with_locale` 保持不变。

## 测试

- 驱动模板编译并覆盖 zh-CN/en-US。
- 覆盖全部可选区块同时存在和全部省略的页面。
- 覆盖请求内容、属性、锚点和代码摘录的转义。
- 覆盖 `prefix_view` 自环、多节点环和悬空引用，必须快速返回错误。
- 保留报告文件路径、原子写入和双格式发布测试。
- 使用现有三二进制端到端脚本验证真实产物。
