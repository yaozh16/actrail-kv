<!-- 本文件定义 v0.1.1 Session 前缀延续与同模板有界多位点的实现设计。 -->
# Session 前缀延续与多位点设计

## 设计边界

v0.1.1 保留现有模板分析主链，新增两个相互独立的分析通道：

```text
CapturedRequest
  ├─ 现有模板通道
  │    → cohort → template → direct defect → Top K
  │                         └→ conditional local sites
  └─ Session 通道
       → timeline → adjacent transition → history sites
```

- Session transition 表达相邻请求之间唯一有效的前缀边界。
- direct defect 保持 v0.1.0 第一个 `P1 / X / P2` 的语义。
- conditional local site 表达 direct defect 之后的局部 `X → P2` 机会，不进入现有 Top K。
- 三类结果可以通过稳定逻辑位置关联，不合并分数或字节统计。

## 输入协议

`CapturedRequest` 顶层新增：

```text
session_id?: string
```

Receiver 使用可选 Header `X-Actrail-Session-Id` 接收该值。`session_id` 不进入 payload、comparison metadata 或 comparison group。

一个 `session_id` 在单个语料中全局标识一条线性请求流。没有该字段时不构造时间线；相同 ID 下的并发分支不在本版本恢复。

Analyzer 的 request identity 在计算 canonical digest 时排除 `session_id`，使关系元数据不改变既有模板和缺陷身份。重复请求仍通过现有 occurrence 后缀保留频次。

## Session 分析

### Observation 与排序

Session observation 保留：

```text
session_id
request_id
captured_at
input_line
source?
boundary dimensions
projection result
```

按 `session_id` 分组，再按 `(captured_at, input_line)` 稳定排序。时间相同的 bucket 只使用 `input_line` 保证确定性，所有触及该 bucket 的 transition 标记为顺序歧义。

时间线基于完整 Session，不使用 comparison time window。endpoint、model、deployment、context schema、agent、KV namespace、dialect 或 adapter revision 变化时输出不可比 boundary。投影失败的 observation 保留为断链点，不得跨过失败请求连接前后成功请求。

### Append-aware prefix

Session comparator 使用开放投影视图：

- 保留结构开始标记；
- 忽略 `messages:end`、`tools:end` 等集合结束标记；
- 按 unit kind、alignment key、层级和 content 逐单元比较；
- 单元 content 不同时计算合法 UTF-8 边界上的最长公共字节前缀；
- 不拼接裸字符串，避免跨 unit 边界形成虚假相等。

每个相邻 transition 只产生一个 prefix frontier，并分类为：

```text
identical
normal_append
prefix_truncated
history_changed
incomparable_boundary
ambiguous_order
unanalyzable_boundary
```

正常追加的新消息或动态 tool result 不形成 history site。只有已存在历史被改写、删除、截断或重排时才形成可聚合事实。

### Session artifact

`AnalysisResult` 新增独立 `session_analysis`：

```text
SessionAnalysis
├─ session_record_count
├─ timelines[]
│  ├─ session_id
│  ├─ requests[]
│  └─ transitions[]
└─ history_sites[]
```

可比较 transition 使用 tagged outcome 保存指标，避免无效的可选字段组合。指标为：

```text
previous_observable_bytes
current_observable_bytes
preserved_prefix_bytes
prefix_retention_ratio
invalidated_previous_suffix_bytes
```

history site 只引用 `history_changed` 或 `prefix_truncated` transition，按不包含 session ID、request ID、时间和动态值的逻辑位置聚合。单次事实可以展示，至少重复两次才生成优化 insight。

## 同模板多位点

### 输出模型

现有第一个 episode 继续映射为 `ContextDefect` 并参与 Top K。`AnalysisResult` 新增 `conditional_local_sites[]`，后续 episode 映射为独立的条件性局部位点；它复用 mismatch facts、recovered stable、affected count、confidence 和 source evidence，但只记录 episode-local stable prefix，不声明实际全局公共前缀，也不参与 Top K。

每个模板最多输出四个 episode，其中最多一个 direct defect、最多三个 conditional local sites。本版本不新增 CLI 或配置项。

### 单链扫描

扫描器维护：

```text
cursor
active_member_ids
local_p1_start
```

算法必须满足：

1. 从 cursor 查找 active members 的下一个真实 mismatch。
2. 查找满足既有支持率、支持数、稳定字节和 anchor 门槛的 recovery。
3. 首次达标位置记录为最小 identity anchor；继续扩展到下一个真实差异前的最大连续稳定区作为展示 P2。
4. 输出 episode 后令 `active_member_ids = recovery supporters`，但支持率分母保持原模板成员总数。
5. cursor 移到最大 P2 结束位置，确保严格递增和 P2 不重叠。
6. active members 在某 slot 上已经相同时，该 slot 不得再次形成 mismatch。
7. 没有合格 P2 的尾部变化不输出 episode。

扫描只沿一条保守 supporter chain 前进，不枚举少数成员分支，避免组合爆炸和重叠计量。

### 身份与计量

- episode identity 使用 X 的逻辑形状和最小资格 anchor，不使用最大 P2 尾部，避免后续内容变化导致前面 ID 漂移。
- direct defect 保持现有全局 P1 指标。
- conditional local site 的 local prefix 从上一 P2 开始计算，不包含更早已经分歧的内容。
- 最大 P2 用于证据和局部 blocked bytes；最小 anchor 用于稳定身份。
- 同模板 episode 不产生总收益，不相加 score、affected count 或 blocked bytes。
- Reporter 将 conditional sites 放在对应模板下展示，不混入 Top K。

## 公共产物与兼容

- `analysis.json` schema 升级为 `0.1.1`。
- v0.1.1 Analyzer 读取所有合法 v0.1.0 capture artifact。
- v0.1.1 Reporter 只接受当前 schema；旧 analysis artifact 由新 Analyzer 重新生成。
- `AnalysisResult` 新增 `session_analysis` 和 `conditional_local_sites`，现有 `templates / defects / top_k` 的语义保持不变。
- 所有新增 DTO 使用严格未知字段拒绝、确定性顺序和稳定 ID。

## 模块布局

```text
artifacts/src/
├─ captured_request.rs
├─ analysis_result.rs
└─ session_analysis.rs

analyzer/src/diagnosis/
├─ episode/
│  ├─ locator.rs
│  └─ scanner.rs
└─ session/
   ├─ mod.rs
   ├─ model.rs
   ├─ prefix.rs
   ├─ timeline.rs
   ├─ aggregate.rs
   └─ tests.rs

reporter/src/
├─ result_loader/
│  ├─ mod.rs
│  ├─ defect.rs
│  ├─ session.rs
│  └─ common.rs
├─ html_report_renderer.rs
└─ session_report_renderer.rs
```

任何现有文件因新增职责接近 500 行时，先拆为目录模块再继续实现。

## 实施顺序

1. 扩充 capture 与 analysis artifact，升级 schema。
2. Receiver 接收 Session Header，loader 保留关系元数据并稳定 request identity。
3. 实现 append-aware prefix comparator 和 Session timeline。
4. 实现 history site 聚合并接入 pipeline。
5. 拆出 episode scanner，实现最多四个 episode 的单链扫描与 conditional local sites。
6. 扩充 Reporter 强校验和展示。
7. 更新累计架构、协议、概念与代码布局文档。
8. 完成单元、性质、规模、E2E、格式和静态检查。

## 设计不变量

- `session_id` 不参与请求结构比较与模板分组。
- 一个 transition 只有一个有效 prefix frontier。
- 新追加动态内容不是历史破坏。
- Session 时间线不跨失败 observation，也不被 comparison time window 切断。
- episode cursor 严格递增，active supporters 只减不增，P2 互不重叠。
- direct defect 与 conditional local site 分层展示，所有通道禁止重复累计影响。
