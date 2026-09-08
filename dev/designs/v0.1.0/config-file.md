<!-- 本文件定义 analyze/receiver 配置文件的模块落点、JSON 契约与合并规则。 -->
# 配置文件化设计

## 模块落点

```text
analyzer/src/config.rs          # AnalyzeConfig：17 项分析参数，JSON 读写与校验
receiver/src/config.rs          # ReceiverConfig：监听地址与请求体上限
cmd/analyze/src/main.rs         # --config <path>，显式 CLI 参数覆盖文件
cmd/receiver/src/main.rs        # --config <path>、--listen、--max-payload-bytes
docs/configuration.md           # 参数表与 JSON 示例
```

## JSON 契约

两个 Config 均使用 serde JSON，字段名与既有 `AnalysisOptions` 快照字段一致（snake_case）；`deny_unknown_fields` 拒绝未知字段。`to_json` 输出可重新 `from_json` 的完整配置。

### AnalyzeConfig（与 `run.options` 快照一一对应）

`text_similarity_threshold` 的算法用途已失效，字段仍为配置与结果兼容保留；当前语义见 [v0.1.2 配置契约](../v0.1.2/implementation.md#配置契约)。以下示例保留本版本序列化记录。

```json
{
  "top_k": 20,
  "comparison_window_seconds": 3600,
  "min_template_members": 3,
  "stable_span_support_ratio": 0.8,
  "min_stable_support": 3,
  "min_blocked_stable_bytes": 64,
  "min_exact_anchor_bytes": 24,
  "text_similarity_threshold": 0.8,
  "template_compatibility_threshold": 0.68,
  "max_dynamic_coverage_ratio": 0.35,
  "max_candidates_per_request": 128,
  "max_projection_units": 512,
  "max_text_unit_bytes": 1048576,
  "max_payload_bytes": 8388608,
  "max_alignment_cells": 2000000,
  "max_total_alignment_cells": 64000000,
  "max_records": 1000000
}
```

### ReceiverConfig

```json
{
  "listen": "127.0.0.1:8080",
  "max_payload_bytes": 8388608
}
```

## 合并规则

```text
编译默认值 < JSON 配置文件 < 显式 CLI 参数
```

CLI 参数改为 `Option`，显式传入时覆盖文件；未传时回退文件，文件缺省时回退默认值。

## 校验

- 比例类字段必须落在 `[0,1]` 且有限；整数预算类字段必须大于零。
- `max_alignment_cells` 与 `max_total_alignment_cells` 放开后由用户按机器内存自行设定；分配发生在单次 LCS，内存约为 cells × 8 字节。

## 兼容性

- 不传 `--config` 时 analyze 的 `run.options` 快照与旧版逐字段一致。
- receiver 无配置文件时监听地址与请求体上限保持旧值。
- 已失效决策索引：`text_similarity_threshold` 不再作为有效算法阈值，兼容保留策略见 [v0.1.2 实现设计](../v0.1.2/implementation.md#已失效决策索引)。
