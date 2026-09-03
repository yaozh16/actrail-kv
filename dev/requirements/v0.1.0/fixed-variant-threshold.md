<!-- 本文件记录 fixed_variants 阈值外部化需求。 -->
# fixed_variants 阈值外部化需求

## 背景

`facts/content.rs` 用魔法数判定“少数固定版本”：变体数 ≤3 且少于成员数。不可配置且与其它阈值口径不一致。

## 目标

新增 `fixed_variant_max` 参数（默认 3），进入 AnalysisOptions、JSON 配置、`run.options` 快照与文档；旧 analysis.json 兼容。

## 验收

- `fixed_variant_max=1` 时 2 版本 fixture 判为 `content_variation`；默认 3 时判为 `fixed_variants`；
- 示例配置与 configuration 文档含该字段。
