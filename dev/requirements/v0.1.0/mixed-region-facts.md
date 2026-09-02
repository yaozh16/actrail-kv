<!-- 本文件记录同一 X 区域同时存在内容变体与缺失变体时的事实输出要求。 -->
# 混合成员区域事实要求

## 背景

`docs/concepts/context_defect.md` 与 `dev/requirements/v0.1.0/analyzer-rewrite.md` 要求：同一 X 区域同时存在内容变体与缺失变体时只形成一个问题，且事实可以同时包含内容变化与插入/缺失。

当前实现中，`facts/content.rs` 把缺失变体也纳入内容 shape 比较，导致有成员缺失该区域时内容事实被整体抑制，pattern 退化为 `insertion_deletion`。

## 验收标准

- 同一模板中部分成员在 X 区域缺失、其余成员填入不同内容时，只产生一个问题。
- 该问题的 pattern 为 `mixed`。
- 该问题同时携带 `content_variation`（或 `fixed_variants`）与 `insertion_deletion` 事实。
- 内容事实只依据“区域实际存在且可绑定”的成员判定；缺失或不可绑定成员只贡献序列证据。
