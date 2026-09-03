<!-- 本文件是 v0.1.1 设计索引，链接本版本需求、实现设计与累计架构文档。 -->
# v0.1.1 实现设计索引

## 需求与验收

- [v0.1.1 开发目标](../../requirements/v0.1.1/target.md)
- [feat/hx-development 特性集成要求](../../requirements/v0.1.1/hx-feature-integration.md)
- [客户能力说明文档要求](../../requirements/v0.1.1/customer-capabilities.md)

## 实现设计

- [Session 前缀延续与多位点设计](session-prefix-and-multi-site.md)

## 累计架构文档

- [代码布局](../../../docs/architectures/code-layout.md)
- [文档布局](../../../docs/architectures/doc-layout.md)
- [Analyzer 流水线](../../../docs/architectures/analyze/pipeline.md)
- [Analyzer 输出](../../../docs/architectures/analyze/output.md)
- [客户能力说明](../../../docs/capabilities.md)

## 失效设计索引

- `feat/hx-development` 的 Hirschberg 文本 LCS 因无法保持重复字符下的 canonical tie-break，本版本保留现有有界矩阵实现；依据见[特性集成要求](../../requirements/v0.1.1/hx-feature-integration.md)。
