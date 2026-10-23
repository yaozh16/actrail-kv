<!-- 本文件定义 report locale 工厂、注册表、快照和 renderer 接入设计。 -->
# Report Locale 设计

## 模块结构

```text
reporter/src/locale/
├── mod.rs          # Locale、namespace 常量与公共 API
├── factory.rs      # 全局当前 locale、初始化、切换、快照
├── registry.rs     # locale/namespace/key 注册与完整校验
├── snapshot.rs     # 不可变快照和模块句柄
├── message.rs      # get、命名参数 format 与错误
└── catalogs/
    ├── mod.rs
    ├── zh_cn.rs
    └── en_us.rs
```

每个 locale 目录通过 namespace 表注册自己的子表。物理文件可以按规模继续拆分，
但 namespace 和 key 契约保持稳定。

## 全局配置

`LocaleFactory` 使用进程级线程安全配置保存当前 locale：

```text
init(locale)
set_locale(locale)
snapshot()
create(namespace)
```

未初始化时默认 `zh-CN`。`set_locale` 只更新全局当前值；快照不引用可变全局状态。

## 快照与句柄

```text
LocaleSnapshot
  - locale
  - create(namespace) → LocaleModule

LocaleModule
  - locale
  - namespace
  - get(key)
  - format(key, named_params)
```

报告导出入口在开始阶段创建一个快照，并将同一快照传给 HTML 和 Markdown renderer。
renderer 不再读取全局 locale。

## 注册表

每个 locale 注册一组 `NamespaceCatalog`：

```text
locale
  namespace
    key → message
```

规则：

- namespace 必须使用已注册常量。
- key 在 namespace 内唯一。
- 所有 locale 的 namespace/key 集合一致。
- 启动测试和单元测试校验注册完整。
- 运行时缺失 key 返回错误。

## 文案格式

- `get(key)`：无参数固定文案。
- `format(key, params)`：使用 `{name}` 命名参数。
- 只解析 locale 模板中的占位符；参数值按纯文本原样插入，不再次解析。
- 缺少参数、重复参数、未闭合或非法占位符必须返回错误。
- 数量、百分比、轮次等可信标量可直接作为参数。
- 不可信请求内容必须由目标格式的 renderer 转义或代码化；`format` 本身不是转义边界。
- locale 文案不包含 HTML 或 Markdown 标记。

## Renderer 接入

HTML：

```text
render_html(result, locale_snapshot)
```

Markdown：

```text
render_markdown(result, locale_snapshot)
```

HTML 的 `lang` 使用 `locale.code()`；Markdown 保持纯 GFM。
部分 renderer 使用同一 namespace 和 key，避免两种格式的 UI 文案漂移。

`write_reports`、单 HTML 的 `write_report` 和 CLI 均在报告入口获取一次全局 locale
快照。无 locale 参数的 renderer 入口仅作为固定兼容接口使用。

## CLI

```text
--locale zh-CN
--locale en-US
```

解析阶段允许大小写差异与 `_`/`-` 等价归一化。未注册 locale 直接失败。
CLI help 文案首期保持固定英文，避免帮助输出依赖 locale 初始化。

## 测试

1. 所有 locale 的 namespace/key 集合一致。
2. key 重复、空 namespace、未知 key 被拒绝。
3. `set_locale` 后已有快照不变化，新快照切换语言。
4. HTML `lang` 随 locale 变化。
5. HTML/Markdown 同一次导出语言一致。
6. `en-US` 固定标签为英文，原始请求内容和 insights 保持原文。
7. 默认 `zh-CN` 输出内容兼容。
