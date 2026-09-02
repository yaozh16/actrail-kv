<!-- 本文件记录按统一 P1/X/P2 模型彻底重写 analyzer 的目标、边界与验收要求。 -->
# Analyzer 统一缺陷模型重写要求

## 状态

- 已批准开发。
- 不兼容旧 analyzer 内部模型和旧 `analysis.json`；v0.1.0 尚未 ready，不实现兼容层。
- 概念基准为 [`docs/concepts/context_defect.md`](../../../docs/concepts/context_defect.md)。
- 流水线基准为 [`docs/architectures/analyze/pipeline.md`](../../../docs/architectures/analyze/pipeline.md)。

## 目标

从同一 comparison group 的完整 LLM 请求中抽取模板成员符号序列，统一定位 `P1 / X / P2` 前缀破坏区域，直接跨模板成员聚合变体，输出可解释的结构证据、优化启示和稳定 Top K。

## 输入分组

`CapturedRequest` 必须显式提供 comparison metadata：

```text
comparison
├─ endpoint_key              required
├─ agent_key?                optional dimension
├─ model_deployment_key?     optional dimension
└─ kv_namespace?             optional; present means mandatory isolation
```

`time_window` 由 analyzer 运行配置给出，`model` 从 payload 提取，`context_schema_key` 由 adapter 产生。不得把 `source` 推断成 endpoint。

## 核心要求

- receiver 只接收并持久化 envelope，不承担模板或缺陷逻辑。
- 模板阶段保留每个成员到共同模板坐标的完整映射。
- 每个请求生成正式的一等符号序列，不允许 diagnosis 回到业务文本重新选择 cause。
- 一个 `P1 / X / P2` 区域只生成一个问题；内容、scope 和等价证据只解释该问题。
- `X` 支持同一位置内容变化、插入/缺失、移动/重排及同一区域的混合成员变体。
- `P2` 必须是明确、连续、支持度与长度达标的稳定区域；没有 `P2` 不报告。
- 每个模板成员在同一问题的变体统计中恰好计数一次。
- 聚合身份不得包含旧 cause、文案、动态值、成员 ID、绝对 unit index 或截断摘录。
- ranking 只负责已唯一化问题的保守评分和确定性排序，不负责猜测根因。
- report 直接展示 comparison group、P1、X 变体、P2、影响和中文优化启示。

## 必须验收

- comparison group：time window、endpoint、model/deployment、context schema、可选 agent、可选 KV namespace 正确隔离。
- 同一位置变化：高基数动态值、少数固定变体、字符串内部槽位均形成同一结构模型。
- 等价表示：JSON object 键序/空白不同可形成等价证据；数组重排和值变化不可。
- 序列变化：插入、缺失、移动、唯一身份重排均可识别；重复或歧义身份不得声称重排。
- 混合成员：同一 X 区域同时存在内容变体与缺失变体时只形成一个问题。
- 正常动态后缀、不同模板、跨 comparison group、非模型可见字段变化保持零问题。
- `affected_count = comparable_count - largest_variant_count`，禁止请求对放大。
- P2 门槛、UTF-8 边界、完整动态区域、重复稳定锚点均有边界测试。
- 输入顺序、成员顺序、展示文案和全体稳定 prelude 变化不改变逻辑问题身份与 Top K。
- 10k 同质语料、有界 alignment 和总资源预算测试通过。
- 三个二进制 E2E、report HTML escape、debug/release tests、Clippy `-D warnings` 全部通过。
