<!-- 本文件记录候选文本相似度对插入/删除错位鲁棒的需求。 -->
# 候选文本相似度位移鲁棒需求

## 背景

`bounded_byte_similarity` 当前按等距字节位置采样 4 字节 shingle；两段文本发生插入/删除后，差异点之后的所有采样窗口错位，稳定长尾几乎全部失配。真实 case（不同长度 Workspace、JSON 空白/键序、有/无额外块）在默认 `template_compatibility_threshold=0.68` 下无法形成 cohort，导致缺陷漏报。

## 目标

- 候选文本相似度对插入/删除造成的位移鲁棒：内容相同、仅局部增删/重排文本的相似度明显高于错位采样。
- 保持确定性、有界预算与现有比较语义（结构兼容、动态覆盖率不变）。
- 默认阈值下 workspace/policy/json/insert 类真实 case 都能形成 cohort 并检出对应 fact。

## 验收

- 同一文本仅在中间插入/删除 2~40 字节时，相似度从当前约 0.55 提升到 ≥0.9（等长 case 保持 1.0）。
- 默认配置下对 `v3-cases/case-{workspace,policy,json,insert}.ndjson` 分析：workspace/json 出 `content_variation`，policy 出 `fixed_variants`，insert 出缺陷。
- 既有 acceptance/properties/scale 测试全绿。
