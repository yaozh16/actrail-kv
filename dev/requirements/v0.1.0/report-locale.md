<!-- 本文件记录 actrail-kv-report 的 locale 工厂、模块子表与快照需求。 -->
# Report Locale 需求

## 背景

HTML 和 GFM Markdown 报告当前含大量中文固定文案。不同显示环境可能使用不同语言，
且 renderer 内分散的字符串无法保证两种格式和多个 locale 的一致性。

## 目标

1. 新增 `--locale`，首期支持 `zh-CN` 和 `en-US`，默认 `zh-CN`。
2. locale 同时作用于 HTML 和 Markdown 的全部固定 UI 文案。
3. 通过全局 `LocaleFactory` 初始化、切换和创建不可变语言快照。
4. 通过点分层级 namespace 创建模块子表，模块内直接按短 key 查询。
5. 同一次报告导出只使用一个 locale 快照，HTML 与 Markdown 不得混用语言。
6. 现有 HTML/Markdown 报告内容、格式选项和输出路径规则保持兼容。

## 使用模型

```rust
LocaleFactory::init(Locale::ZhCn)?;

let locale = LocaleFactory::snapshot()?;
let defects = locale.create("report.defects")?;
let title = defects.get("section_title")?;
let warning = defects.format(
    "affected_requests",
    &[("count", "2")],
)?;
```

`LocaleFactory::create(namespace)` 等价于先获取当前全局 locale 快照，再创建模块句柄。

## 快照语义

- `init` 设置全局默认 locale。
- `set_locale` 修改后续快照使用的 locale。
- 当前报告导出开始时获取一次快照。
- 已创建的模块句柄在生命周期内语言固定。
- 中途修改全局 locale 不影响正在导出的报告，只影响下一次导出。

## Namespace

首期 namespace：

```text
report.common
report.prefix_reuse
report.top_k
report.defects
report.sessions
report.cache
report.prefix_tree
cli.report
```

key 只要求在所属 namespace 内唯一。例如以下三个 `title` 互不冲突：

```text
report.defects.title
report.sessions.title
report.cache.title
```

## 文案边界

纳入 locale：

- 标题、章节名、表头、说明、空态；
- 缺陷类型、事实、优化启示标签；
- session 事件、刷新警告和口径；
- 缓存真实/估算标签；
- 前缀树图例和字段；
- locale 相关错误。

不得翻译：

- `analysis.json` 字段名和枚举值；
- `P1 / X / P2`、`TTFT`、ID、路径和技术标识；
- 请求原文、工具内容、摘录和内容样本；
- analyzer 写入 `analysis.json` 的 `insights[].summary/detail`；
- 数值、排序和机器契约。

选择 `en-US` 只保证报告固定 UI 文案为英文；请求内容和 analyzer insights 仍可能是中文。
HTML/Markdown 文件继续使用 UTF-8，Markdown 不增加 HTML meta 或 BOM。

## 错误规则

- 未注册 locale 必须拒绝，并列出支持项。
- 未注册 namespace/key 必须报错，不能返回空字符串。
- 缺失 key 不得静默回退到中文。
- locale、namespace 和 key 注册必须在测试中校验完整。

## 验收

- 默认 `zh-CN` 报告与现有内容等价。
- `en-US` 下 HTML 和 Markdown 固定 UI 文案均为英文。
- 同一次双格式导出使用同一 locale 快照。
- `set_locale` 不影响已有句柄。
- HTML 的 `lang` 属性随 locale 更新。
- 两种 locale 的 namespace/key 集合完全一致。
- 请求原文和 analyzer insights 不被改写。
