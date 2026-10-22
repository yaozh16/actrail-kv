<!-- 本文件记录 actrail-kv-report 默认生成 HTML 与 GFM Markdown 双报告的需求。 -->
# HTML 与 Markdown 双报告需求

## 背景

`actrail-kv-report` 当前只生成自包含静态 HTML，适合浏览器审阅；Markdown 更适合代码仓库、
缺陷跟踪系统和纯文本评审流程。HTML 与 Markdown 必须来自同一份 `analysis.json`，
保持报告信息内容一致，避免维护两套分析逻辑。

## 目标

1. 保留现有 HTML 报告逻辑和产物内容不变。
2. 新增纯 GitHub Flavored Markdown 报告，不嵌入 HTML、CSS、SVG 或 JavaScript。
3. 默认一次生成同名的 HTML 与 Markdown 两份报告。
4. 新增 `--format html|md`：
   - 不传：生成两份；
   - `html`：只生成 HTML；
   - `md`：只生成 Markdown。
5. 两份报告消费同一份已加载、已校验的 `AnalysisResult`。

## 内容等价

Markdown 必须覆盖 HTML 的全部信息内容：

- 标题、输入/分析/跳过数量和缺陷摘要；
- 结构复用率、目标、理论上界、三段字节构成和结论；
- Top K 的顺序、位置、阻断字节、受影响请求数、得分、建议和证据关联；
- 全部缺陷详情，包括 comparison group、P1、X 变体与 facts、P2、指标与 insights；
- 会话上下文刷新概览、首次刷新、逐 session 与逐事件证据；
- KV 缓存命中率、时延、真实/估算口径和环比；
- 结构代理与真实命中对照；
- 模板前缀树、复用分布、成员衰减、节点、变体、成员和缺陷关联；
- 所有原有空态、限制说明和口径说明。

Markdown 不复刻 HTML 的像素、颜色、CSS 或交互式折叠。图形改用 GFM 表格或列表表达其完整数据，
因此“内容不改变”指信息内容等价，不是视觉样式一致。

## CLI 与路径

`--output` 继续必填，且不得与输入指向同一文件。

未传 `--format` 时，将输出路径归一化为报告基础路径：

```text
report.html → report.html + report.md
report.md   → report.html + report.md
report      → report.html + report.md
```

只传格式时，严格写入 `--output` 指定路径：

```bash
actrail-kv-report --input analysis.json --output report.html --format html
actrail-kv-report --input analysis.json --output report.md --format md
```

`.html` 和 `.md` 后缀按不区分大小写识别；其他后缀在默认双报告模式下视为基础路径。

## Markdown 约束

- 方言为 GFM，支持标准表格和标题锚点。
- 不生成 `<details>`、`<style>`、`<svg>` 等原始 HTML。
- 请求内容、路径、摘录和 ID 中的 Markdown 特殊字符必须转义或放入安全代码片段。
- 不因展示层转义改变原始值的可辨识内容。
- 文件名、标题和段落保持 UTF-8。

## 验收

- 默认调用同时产生 `.html` 和 `.md`，HTML 字节与改造前同一输入的结果一致。
- `--format html` 只产生指定 HTML 路径，不产生 Markdown。
- `--format md` 只产生指定 Markdown 路径，不产生 HTML。
- Markdown 包含全部报告区块和关键字段，空字段区块按原规则省略。
- Markdown 不存在原始 HTML、脚本或远程资源。
- Markdown 表格中的 `|`、反引号、换行和标题字符不会破坏文档结构。
- 双报告任一渲染失败时，不发布只有一份完整结果的双格式成功产物。
