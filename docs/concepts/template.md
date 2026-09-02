<!-- 本文件用 Coding Agent 的多轮请求示例解释模板及其 analysis.json 映射。 -->
# 什么是请求模板？

在 Actrail KV 中，模板是一组整体结构相近的 LLM 请求。analyzer 从完整 HTTP request payload 中自动归纳模板；它不是开发者手写的 prompt 模板，也不是模型内部 token/KV cache 条目。

## 两个 Coding Agent 请求

`T` 表示模板中的共同结构或稳定块；相同编号表示两个请求中被识别为同一类块，但下图没有共享节点或跨请求连线。`S` 表示槽位的一次填充；`D` 表示当前未形成稳定模板块的动态内容。

```text
rqst-1：修复 HTTP client 重试超时
├── model: coding-model                                    [T0]
├── tools
│   ├── tools[0]: search_code(query)                       [T1]
│   ├── tools[1]: read_file(path)                          [T2]
│   ├── tools[2]: apply_patch(path, patch)                 [T3]
│   └── tools[3]: run_tests(command)                       [T4]
└── messages
    ├── messages[0]
    │   ├── role: system                                   [T5]
    │   └── content
    │       ├── 你是一个 Coding Agent。                     [T6] @ 这里真的合理吗？为什么"当前工作区："不在模板里？
    │       ├── 当前工作区：/workspace/service-a            [S1 = /workspace/service-a]
    │       └── 修改前先读代码，修改后必须运行相关测试。     [T7]
    ├── messages[1]
    │   ├── role: user                                     [T8]
    │   └── content: 修复 HTTP client 重试超时              [D]
    ├── messages[2]
    │   ├── role: assistant                                [T9]
    │   └── tool_calls                                     [S2 = 整个 tool_calls JSON]
    │       └── tool_calls[0]
    │           ├── function.name: search_code
    │           └── function.arguments.query: retry_timeout
    ├── messages[3]
    │   ├── role: tool                                     [T11]
    │   ├── tool_call_id                                   [T12]
    │   └── content: src/http/client.rs:87 ...             [D]
    ├── messages[4]
    │   ├── role: assistant                                [T9]
    │   └── tool_calls                                     [S3 = 整个 tool_calls JSON]
    │       └── tool_calls[0]
    │           ├── function.name: read_file
    │           └── function.arguments.path: src/http/client.rs
    ├── messages[5]
    │   ├── role: tool                                     [T11]
    │   ├── tool_call_id                                   [T14]
    │   └── content: <client.rs 的实际源码>                 [D]
    ├── messages[6]
    │   ├── role: assistant                                [T9]
    │   └── tool_calls                                     [S4 = 整个 tool_calls JSON]
    │       └── tool_calls[0]
    │           ├── function.name: apply_patch
    │           └── function.arguments
    │               ├── path: src/http/client.rs
    │               └── patch: <本次代码补丁>
    ├── messages[7]
    │   ├── role: tool                                     [T11]
    │   ├── tool_call_id                                   [T16]
    │   └── content: patch applied                         [D]
    ├── messages[8]
    │   ├── role: assistant                                [T9]
    │   └── tool_calls                                     [S5 = 整个 tool_calls JSON]
    │       └── tool_calls[0]
    │           ├── function.name: run_tests
    │           └── function.arguments.command: cargo test http
    ├── messages[9]
    │   ├── role: tool                                     [T11]
    │   ├── tool_call_id                                   [T18]
    │   └── content: 18 passed; 0 failed                   [D]
    └── messages[10]
        ├── role: assistant                                [T9]
        └── content: 已修复重试超时并通过 18 项测试         [D]

rqst-2：修复配置解析空值崩溃
├── model: coding-model                                    [T0]
├── tools
│   ├── tools[0]: search_code(query)                       [T1]
│   ├── tools[1]: read_file(path)                          [T2]
│   ├── tools[2]: apply_patch(path, patch)                 [T3]
│   └── tools[3]: run_tests(command)                       [T4]
└── messages
    ├── messages[0]
    │   ├── role: system                                   [T5]
    │   └── content
    │       ├── 你是一个 Coding Agent。                     [T6]
    │       ├── 当前工作区：/workspace/service-b            [S1 = /workspace/service-b]
    │       └── 修改前先读代码，修改后必须运行相关测试。     [T7]
    ├── messages[1]
    │   ├── role: user                                     [T8]
    │   └── content: 修复配置解析遇到 null 时崩溃           [D]
    ├── messages[2]
    │   ├── role: assistant                                [T9]
    │   └── tool_calls                                     [S2 = 整个 tool_calls JSON]
    │       └── tool_calls[0]
    │           ├── function.name: search_code
    │           └── function.arguments.query: parse_config
    ├── messages[3]
    │   ├── role: tool                                     [T11]
    │   ├── tool_call_id                                   [T12]
    │   └── content: src/config/parser.rs:42 ...           [D]
    ├── messages[4]
    │   ├── role: assistant                                [T9]
    │   └── tool_calls                                     [S3 = 整个 tool_calls JSON]
    │       └── tool_calls[0]
    │           ├── function.name: read_file
    │           └── function.arguments.path: src/config/parser.rs
    ├── messages[5]
    │   ├── role: tool                                     [T11]
    │   ├── tool_call_id                                   [T14]
    │   └── content: <parser.rs 的实际源码>                 [D]
    ├── messages[6]
    │   ├── role: assistant                                [T9]
    │   └── tool_calls                                     [S4 = 整个 tool_calls JSON]
    │       └── tool_calls[0]
    │           ├── function.name: apply_patch
    │           └── function.arguments
    │               ├── path: src/config/parser.rs
    │               └── patch: <本次代码补丁>
    ├── messages[7]
    │   ├── role: tool                                     [T11]
    │   ├── tool_call_id                                   [T16]
    │   └── content: patch applied                         [D]
    ├── messages[8]
    │   ├── role: assistant                                [T9]
    │   └── tool_calls                                     [S5 = 整个 tool_calls JSON]
    │       └── tool_calls[0]
    │           ├── function.name: run_tests
    │           └── function.arguments.command: cargo test config
    ├── messages[9]
    │   ├── role: tool                                     [T11]
    │   ├── tool_call_id                                   [T18]
    │   └── content: 11 passed; 0 failed                   [D]
    └── messages[10]
        ├── role: assistant                                [T9]
        └── content: 已修复 null 崩溃并通过 11 项测试       [D]
```

两棵树中的节点始终属于各自请求。相同 T 编号只说明这些独立节点被归纳为同类共同结构；`S1` 到 `S5` 表示同类位置在两次请求中的不同填充值。

树中展开了 `tool_calls[0].function`，是为了展示真实请求层级。当前 adapter 实际把整个 `tool_calls` canonical JSON 投影成一个单元，因此 S2 到 S5 的 `source.json_path` 指向相应 message 的 `tool_calls`，不会分别指向 `function.name` 或某一个 argument。

## 两个办公助手 Agent

下面是另一个独立模板。它描述办公助手先查询参会人的空闲时间，再创建会议并发送通知。编号从 T20/S20 开始，表示它与前面的 Coding Agent 不是同一个模板。

```text
rqst-3：为销售部安排周报评审
├── model: office-model                                    [T20]
├── tools
│   ├── tools[0]: search_calendar(attendees, time_range)   [T21]
│   ├── tools[1]: create_event(title, attendees, time)     [T22]
│   └── tools[2]: send_email(to, subject, body)            [T23]
└── messages
    ├── messages[0]
    │   ├── role: system                                   [T24]
    │   └── content
    │       ├── 你是公司办公助手。                          [T25]
    │       ├── 当前部门：销售部                            [S20 = 销售部]
    │       └── 创建会议前检查空闲时间，创建后发送通知。    [T26]
    ├── messages[1]
    │   ├── role: user                                     [T27]
    │   └── content: 安排 Alice 明天下午评审周报           [D]
    ├── messages[2]
    │   ├── role: assistant                                [T28]
    │   └── tool_calls                                     [S21 = 整个 tool_calls JSON]
    │       └── tool_calls[0]
    │           ├── function.name: search_calendar
    │           └── function.arguments
    │               ├── attendees: [Alice]
    │               └── time_range: 明天下午
    ├── messages[3]
    │   ├── role: tool                                     [T29]
    │   ├── tool_call_id                                   [T30]
    │   └── content: Alice 15:00-16:00 available           [D]
    ├── messages[4]
    │   ├── role: assistant                                [T28]
    │   └── tool_calls                                     [S22 = 整个 tool_calls JSON]
    │       └── tool_calls[0]
    │           ├── function.name: create_event
    │           └── function.arguments
    │               ├── title: 周报评审
    │               ├── attendees: [Alice]
    │               └── time: 明天 15:00
    ├── messages[5]
    │   ├── role: tool                                     [T29]
    │   ├── tool_call_id                                   [T31]
    │   └── content: {event_id: EVT-301, created: true}    [D]
    ├── messages[6]
    │   ├── role: assistant                                [T28]
    │   └── tool_calls                                     [S23 = 整个 tool_calls JSON]
    │       └── tool_calls[0]
    │           ├── function.name: send_email
    │           └── function.arguments
    │               ├── to: [Alice]
    │               ├── subject: 周报评审邀请
    │               └── body: 明天 15:00 会议室 A
    ├── messages[7]
    │   ├── role: tool                                     [T29]
    │   ├── tool_call_id                                   [T32]
    │   └── content: email sent                            [D]
    └── messages[8]
        ├── role: assistant                                [T28]
        └── content: 已创建会议并向 Alice 发送邀请          [D]

rqst-4：为产品部安排发布评审
├── model: office-model                                    [T20]
├── tools
│   ├── tools[0]: search_calendar(attendees, time_range)   [T21]
│   ├── tools[1]: create_event(title, attendees, time)     [T22]
│   └── tools[2]: send_email(to, subject, body)            [T23]
└── messages
    ├── messages[0]
    │   ├── role: system                                   [T24]
    │   └── content
    │       ├── 你是公司办公助手。                          [T25]
    │       ├── 当前部门：产品部                            [S20 = 产品部]
    │       └── 创建会议前检查空闲时间，创建后发送通知。    [T26]
    ├── messages[1]
    │   ├── role: user                                     [T27]
    │   └── content: 安排 Bob 周五上午评审发布计划          [D]
    ├── messages[2]
    │   ├── role: assistant                                [T28]
    │   └── tool_calls                                     [S21 = 整个 tool_calls JSON]
    │       └── tool_calls[0]
    │           ├── function.name: search_calendar
    │           └── function.arguments
    │               ├── attendees: [Bob]
    │               └── time_range: 周五上午
    ├── messages[3]
    │   ├── role: tool                                     [T29]
    │   ├── tool_call_id                                   [T30]
    │   └── content: Bob 10:00-11:00 available             [D]
    ├── messages[4]
    │   ├── role: assistant                                [T28]
    │   └── tool_calls                                     [S22 = 整个 tool_calls JSON]
    │       └── tool_calls[0]
    │           ├── function.name: create_event
    │           └── function.arguments
    │               ├── title: 发布计划评审
    │               ├── attendees: [Bob]
    │               └── time: 周五 10:00
    ├── messages[5]
    │   ├── role: tool                                     [T29]
    │   ├── tool_call_id                                   [T31]
    │   └── content: {event_id: EVT-402, created: true}    [D]
    ├── messages[6]
    │   ├── role: assistant                                [T28]
    │   └── tool_calls                                     [S23 = 整个 tool_calls JSON]
    │       └── tool_calls[0]
    │           ├── function.name: send_email
    │           └── function.arguments
    │               ├── to: [Bob]
    │               ├── subject: 发布计划评审邀请
    │               └── body: 周五 10:00 会议室 B
    ├── messages[7]
    │   ├── role: tool                                     [T29]
    │   ├── tool_call_id                                   [T32]
    │   └── content: email sent                            [D]
    └── messages[8]
        ├── role: assistant                                [T28]
        └── content: 已创建会议并向 Bob 发送邀请            [D]
```

办公助手的两棵树同样互不连接。相同 T 编号只表示同类共同结构；S20 是部门填充值，S21 到 S23 分别是查询日历、创建会议和发送邮件时的完整 `tool_calls` 单元。

## 为什么这会产生 Finding

Coding Agent 的 system content 中，工作区 S1 位于稳定规则 T7 前：

```text
[T6 你是一个 Coding Agent。]
→ [S1 当前工作区：/workspace/service-a 或 service-b]
→ [T7 修改前先读代码，修改后必须运行相关测试。]
```

S1 不同会使公共前缀在此停止，后面的 T7 无法进入更长的共同结构。因此 analyzer 可以报告 `inline_dynamic_slot` 或 `early_variable_content`：它不是要求删除工作区，而是提示业务方评估能否把稳定规则放到工作区信息之前。

办公助手也有同样的结构：部门槽位 S20 位于稳定规则 T26 前。销售部/产品部的差异会提前终止公共前缀，挡住后面的“创建会议前检查空闲时间，创建后发送通知”。

工具结果、补丁内容和最终总结标为 `D`，并不表示它们不重要；它们只是本次 Agent 轨迹特有的内容，没有形成稳定模板块。位于请求末尾的动态内容通常也不会单独产生“挡住后续稳定内容”的 Finding。

模板建立后，analyzer 当前可以识别以下上下文组织问题：

- 较早出现的易变内容挡住后续稳定内容；
- 同一段文本中间的动态槽位挡住稳定后缀；
- 独立动态块被放在稳定块之前；
- 工具定义顺序漂移；
- 同一工具的定义发生微小漂移；
- system prompt 发生微小漂移；
- 对话历史不是只在末尾追加；
- 模型可见 JSON 数据相同，但序列化格式不同。

这些问题的形成条件、具体请求例子和非缺陷反例见[上下文结构缺陷](context_defect.md)。

## 与 `analysis.json` 的关系

| 图中概念 | `analysis.json` 路径 | 含义 |
|---|---|---|
| T 标记所属的一组请求 | `templates[].member_request_ids[]` | 同一模板的成员请求。 |
| model、role 等共同结构 | 比较域、模板成员与 `cohesion` | 参与分组和对齐；当前输出不逐项列出所有短结构标记。 |
| T 标记的稳定文本/工具定义 | `templates[].stable_spans[]` | 达到长度和支持门槛的共同稳定片段。 |
| S 标记的填充值 | `templates[].slots[]` | 动态位置及其不同取值数量。 |
| S1 挡住后续 T7 | `findings[].source`、`cause`、`blocked_stable_bytes` | 问题位置、原因和被阻断的稳定内容。 |
| 优先修复项 | `top_k[]` | 排序后的 Finding。 |

`templates[]` 回答“哪些请求和内容具有共同结构”；`findings[]` 回答“共同结构中哪里组织得不利于复用”；`top_k[]` 回答“先看哪个问题”。完整字段参考见 [Analyze 输出](../architectures/analyze/output.md)，抽取过程见 [Analyze 流水线](../architectures/analyze/pipeline.md)。

## 边界

- 当前工具离线读取一批请求后共同抽取模板；图中的 T/S 标记不是在线模板库查询。
- 稳定块只表示它在当前语料中得到足够支持，不保证永远不变。
- `utf8_bytes`、`blocked_stable_bytes` 和 score 是结构诊断指标，不等于 token 数、真实 KV 命中率或成本收益。
