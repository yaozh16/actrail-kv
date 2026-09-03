<!-- 本文件定义阈值外部化接线。 -->
# fixed_variants 阈值外部化设计

## 接线

```text
AnalysisOptions.fixed_variant_max
  → AnalyzeConfig（serde default=3，旧文件可缺省）
  → AnalysisOptionsSnapshot（serde default=3）
  → DiagnosisOptions.fixed_variant_max
  → facts::analyze(variants, fixed_variant_max)
```

`content.rs` 判定改为：

```text
fixed = signatures.len() <= fixed_variant_max && signatures.len() < nonempty.len()
```

## 兼容

- JSON 配置未写该字段时使用默认值；
- 旧 analysis.json 读取时 `fixed_variant_max` 回落到 3。
