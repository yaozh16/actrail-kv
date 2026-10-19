<!-- 本文件定义会话上下文刷新检测的分析算法、artifact 契约与报告实现边界。 -->
# 会话上下文刷新检测设计

## 现状与设计约束

当前会话分析位于 `analyzer/src/session/lineage.rs`，已经能对相邻请求分类并输出
`append / fork / reorder / reset`、公共前缀、重算字节和 `prefix_cut_bytes`。

现有实现不能直接满足新需求：

1. `session_reports` 按 `session_key + endpoint_key + model` 分组，跨模型或 endpoint 的同一
   session 会被拆成多个报告。
2. `session_rows` 按输入文件顺序构建，没有按会话实际采集时间排序。
3. pipeline 将全部记录与“投影成功后的 sequence 数组”直接 `zip`；只要前面有投影跳过，
   后续 session 就可能关联到错误请求。
4. `prefix_cut_bytes` 表示相邻两次 LCP 的收缩量，不能替代“上一条上下文是否被完整保留”。
5. report 需要关注会话的判断依赖非追加事件或 `prefix_cut_bytes`，没有独立的刷新次数。

本设计先修复会话时间线与记录配对，再在新时间线之上计算逐请求前缀复用度。

## 数据流

```text
requests.ndjson
  → CorpusLoader
  → RequestProjector
  → (CorpusRecord, CacheSequence) 成功投影对
  → session::timeline
       按 session_key 分组
       按 captured_at + input_line 排序
  → session::switch
       相邻对 LCP 与 append/fork/reorder/reset 分类
  → session::reuse
       逐请求 prefix_reuse_ratio 与刷新离群判断
  → SessionReport
  → AnalysisResult.session_reports
  → report.html「会话上下文刷新」
```

会话事件不进入 `defects[]` 与 `top_k[]`。上下文刷新是会话生命周期 warning，
上下文结构 `defect` 是请求模板中的优化问题，两者身份、排序和用户决策边界不同。

## 逐请求指标

对相邻请求 `previous`、`current`：

```text
previous_total_bytes = Σ previous 中内容性单元 content.len()
retained_prefix_bytes = previous 与 current 内容性单元连续公共前缀字节
prefix_reuse_ratio = retained_prefix_bytes / previous_total_bytes
abandoned_prefix_bytes = previous_total_bytes - retained_prefix_bytes
```

规则：

- `previous_total_bytes == 0` 时不计算比值，字段缺省；
- `retained_prefix_bytes == previous_total_bytes` 时为正常连续性，`prefix_reuse_outlier = false`；
- `retained_prefix_bytes < previous_total_bytes` 时为离群点，`prefix_reuse_outlier = true`；
- 离群判断使用整数字节比较，浮点比值只用于展示；
- 不使用上下文窗口、压缩阈值、tokenizer 或自适应统计阈值。

结构起止标记（`messages:start` / `messages:end`）会因追加消息而移动，不参与本指标。
已有 `lcp_bytes` 继续按全部投影单位计算；新指标只统计内容性单元，两者在 artifact 中并存。

`append / fork / reorder / reset` 仍由现有单位序列分类器产生。`prefix_reuse_outlier`
说明是否发生上下文刷新，事件类型说明上下文如何变化，二者不互相替代。

## Artifact 契约

新增字段全部使用 serde default，保持旧版 `analysis.json` 可加载。
`ANALYSIS_SCHEMA_VERSION` 保持 `0.1.0`；analyzer 与 report 按同版本配套发布。
`SessionSwitchEvent` 与 `SessionReport` 因包含 `f64` 比值，不再派生 `Eq`，只保留
`PartialEq`；`SessionEventType` 继续保持 `Eq`。

### `SessionSwitchEvent`

| 字段 | 类型 | 含义 |
|---|---|---|
| `previous_sequence` | integer | 前一条请求在 session 内的一基序号。 |
| `next_sequence` | integer | 当前请求在 session 内的一基序号。 |
| `previous_total_bytes` | integer | 上一条请求内容性上下文总字节。 |
| `retained_prefix_bytes` | integer | 内容性单元连续公共前缀字节。 |
| `prefix_reuse_ratio?` | number，`0..1` | `retained_prefix_bytes / previous_total_bytes`；分母为 0 时缺省。 |
| `abandoned_prefix_bytes` | integer | `previous_total_bytes - retained_prefix_bytes`。 |
| `prefix_reuse_outlier` | boolean | 当前相邻对是否构成上下文刷新。 |
| `domain_changed` | boolean | endpoint、model、deployment、schema、agent 或 namespace 是否变化。 |

已有字段保持原语义：

- `lcp_bytes`、`lcp_units`：公共前缀大小；
- `next_total_bytes`、`recomputed_bytes`：下一条上下文与重算量；
- `stable_after_switch_bytes`、`next_block_runs`：分歧后的结构证据；
- `previous_pair_lcp_bytes`、`prefix_cut_bytes`：相邻两次 LCP 的走势，不用于刷新计数。

### `SessionReport`

| 字段 | 类型 | 含义 |
|---|---|---|
| `context_refresh_count` | integer | `prefix_reuse_outlier = true` 的事件数。 |
| `total_abandoned_prefix_bytes` | integer | 所有刷新事件废弃前缀字节之和。 |
| `domain_change_count` | integer | 发生比较域变化的相邻对数量。 |

`SessionReport.endpoint_key` 与 `.model` 仍表示会话第一条有效投影请求所在域。
比较域变化不会拆分 session，而由事件的 `domain_changed` 标记。

## 分析实现

### 时间线

`SessionRow` 增加：

```text
captured_at
input_line
```

pipeline 在投影循环中直接保存 `(CorpusRecord, CacheSequence)` 对应关系，
再从成功投影对构造 `SessionRow`。禁止对完整记录数组和成功投影结果数组直接 `zip`。

`session::timeline` 只按 `session_key` 建桶，桶内按解析后的 RFC 3339 时间与
`input_line` 排序。输出稳定：

```text
captured_at ASC, input_line ASC, session_key ASC
```

### 分类与连续性

`session::switch` 保留现有 `classify`、`same_observable`、`unique_permutation` 和
run-length 逻辑。每个相邻对同时产出：

- 事件类型；
- 公共前缀 units/bytes；
- 前一条上下文总字节；
- 废弃前缀字节；
- 可测时的前缀复用度；
- 离群布尔值。

### 会话聚合

`session::reuse` 遍历事件计算：

```text
context_refresh_count =
  events.filter(prefix_reuse_outlier).count()

total_abandoned_prefix_bytes =
  Σ events.abandoned_prefix_bytes
```

`session_reports` 与报告使用同一稳定排序：

```text
context_refresh_count DESC
→ total_abandoned_prefix_bytes DESC
→ session_key ASC
```

原有 `prefix_reuse` 汇总继续使用 `lcp_bytes / current_total_bytes` 口径，
回答“当前请求有多少前缀可复用”；新的 session 事件指标回答
“上一条上下文是否被保留”。两个口径不得合并成一个比例。

`prefix_reuse.by_session` 继续遵循 `session_reports` 的顺序。

## 模块落点

`analyzer/src/session/lineage.rs` 承载时间线、分类和会话聚合；结构复用率汇总拆到：

```text
analyzer/src/session/
├── mod.rs          # session API re-export
├── lineage.rs      # 时间排序、相邻对分类、刷新事件与 SessionReport 聚合
└── reuse.rs        # prefix_reuse 汇总与缺陷可恢复上界
```

`artifacts/src/analysis_result.rs` 拆分出 session、cache metric、运行摘要和 prefix reuse 子模块；
`artifacts/src/lib.rs` 的公共 API 与磁盘字段路径保持不变。

`reporter/src/html_report_renderer.rs` 将 session 报告拆到
`reporter/src/html_report_renderer/session.rs`，通用 HTML escape、字节格式化和短 ID
保留在主模块。

## 报告实现

会话区块标题改为“会话上下文刷新与前缀复用”。

概览指标：

- 会话数；
- 请求数；
- 上下文刷新 warning 数；
- 发生刷新的会话数；
- 被废弃前缀总字节。

需要关注区包含 `context_refresh_count > 0` 的会话。会话行展示：

- 会话标识；
- 请求数；
- 刷新次数；
- 最低前缀复用度；
- 废弃前缀字节；
- 现有事件类型标签。

逐事件明细增加：

- 前后请求的 session 序号；
- 前缀复用度；
- 废弃前缀字节；
- warning 标记；
- 比较域变化标记。

报告明确写出：

```text
上下文刷新可能是用户切换任务、compact 或 subagent 等正常行为；
它仍是 KVC 前缀废弃风险，需要人工判断是否接受。
```

旧版 `analysis.json` 没有新字段时：

- 刷新次数回退为 `fork_count + reorder_count + reset_count`；
- 废弃前缀回退为现有 `prefix_cut_bytes` 用于展示；
- 复用度显示 `—`；
- 不伪造离群判断。

旧版 report 二进制读取包含新增字段的新版 `analysis.json` 会因严格 schema 拒绝；
发布时必须同时更新 analyzer 与 report。

## 测试

### Analyzer

1. `A / AB / ABC / ABCD / EF / EFG / EFGH` 产生一个刷新事件，
   位置为 `ABCD → EF`，复用度序列为 `1 / 1 / 1 / 0 / 1 / 1`。
2. append-only session 的 `context_refresh_count = 0`。
3. 同一 session 的请求跨 model 时不拆分报告，事件标记 `domain_changed`。
4. 输入乱序时按 `captured_at` 重排；时间相同按 `input_line` 重排。
5. 中间存在 projection skip 时，session 仍只与自身成功投影请求配对。
6. compact 型 reset 与 subagent 型 fork 都产生 warning，不产生 `defects[]`。
7. 无 `session_key`、单请求 session 和零字节前序不产生伪刷新。
8. 旧版 artifact 缺少新增字段时通过 serde default 解析。

### Reporter

1. 有刷新时展示 warning 数、刷新次数、轮次和废弃字节。
2. 无刷新时显示“未发现上下文刷新”，不渲染警告表。
3. 旧版 artifact 使用事件计数回退，复用度显示 `—`。
4. 所有新增文本字段经过 HTML escape。

### End-to-end

扩展 `examples/cases/session.ndjson` 或新增等价 fixture，使
`receiver → analyze → report` 产物同时覆盖 append、reset、重新追加和刷新 warning。

## 文档维护

实现时同步更新：

- `docs/architectures/analyze/output.md`：新增 artifact 字段与一致性规则；
- `docs/architectures/report/output.md`：刷新 warning 展示契约；
- `docs/architectures/code-layout.md`：session 和 reporter 新模块；
- `docs/concepts/prefix_reuse.md`：区分“上轮上下文保留度”与“当前请求可复用比例”。

本需求不修改 `docs/configuration.md`；没有新增配置项。
