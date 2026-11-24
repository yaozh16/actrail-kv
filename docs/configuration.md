<!-- 本文件是当前可运行版本的配置参考；只记录已实现的参数与默认值。 -->
# 配置参考

## 配置方式

各命令都接受命令行参数。Analyzer 与 Receiver 支持独立的运行配置和采集存储配置。

- 配置优先级：编译默认值 < JSON 配置文件 < 显式 CLI 参数。
- JSON 配置通过 `--config <path>` 传入；未提供时使用编译默认值，行为与旧版本一致。
- 配置包含未知字段或非法数值时拒绝启动。
- 每次分析会把实际生效的算法参数写入 `analysis.json` 的 `run.options`。

## Receiver

```bash
actrail-kv-receiver \
  --config /etc/actrail-kv/receiver.json \
  --listen 127.0.0.1:8080 \
  --output /var/lib/actrail-kv/requests.ndjson
```

| CLI 参数 | 默认值 | 说明 |
|---|---:|---|
| `--listen` | `127.0.0.1:8080` | HTTP 监听地址。默认仅 loopback；跨主机接入应在内网反向代理后显式配置。 |
| `--output` | 与 `--storage-config` 二选一 | 兼容模式的 NDJSON 单文件路径。进程独占该文件，Unix 文件权限为 `0600`。 |
| `--storage-config` | 与 `--output` 二选一 | SQLite 或托管 NDJSON 的配置文件路径。 |
| `--config` | 无 | 可选 JSON 配置；CLI 显式参数覆盖文件。 |
| `--max-payload-bytes` | 来自配置 | 请求体上限覆盖；默认 `8 MiB`。 |

`receiver.json` 支持字段：`listen`、`max_payload_bytes`（默认 `8388608`）。示例见 `examples/receiver.config.example.json`。

Receiver 接受 `POST /requests`，body 必须是完整 JSON object。`X-Actrail-Endpoint-Key` 必填；`X-Actrail-Agent-Key`、`X-Actrail-Model-Deployment-Key`、`X-Actrail-KV-Namespace` 和 `X-Actrail-Source` 可选。前三项 metadata 提供后参与 comparison group，source 只记录采集来源。

## Analyze

```bash
actrail-kv-analyze \
  --config /etc/actrail-kv/analyze.json \
  --input /var/lib/actrail-kv/requests.ndjson \
  --output /var/lib/actrail-kv/analysis.json \
  --top-k 20 \
  --comparison-window-seconds 3600
```

| CLI 参数 | 默认值 | 说明 |
|---|---:|---|
| `--input` | 与 `--storage-config` 二选一 | 兼容模式的 NDJSON 语料文件。 |
| `--storage-config` | 与 `--input` 二选一 | 采集存储配置文件路径。 |
| `--session-key`、`--from`、`--until` | 无 | 托管存储的筛选参数，见[采集存储](#采集存储)。 |
| `--output` | 无，必填 | 分析 JSON 的目标路径，必须位于采集存储之外。 |
| `--config` | 无 | 可选 JSON 配置；显式 CLI 参数覆盖文件。 |
| `--top-k` | `20` | 最终报告保留的聚合 defect 数，必须大于零。 |
| `--comparison-window-seconds` | `3600` | 固定 UTC 时间窗口宽度；只有同一窗口内的请求才相互比较。 |

### JSON 配置参数

analyze 的全部算法阈值与资源预算均可通过 `--config` 的 JSON 覆盖；以下表格列出字段与默认值。不传配置时使用这些默认值。示例见 `examples/analyze.config.example.json`。

| 字段 | 默认值 | 作用 |
|---|---:|---|
| `top_k` | `20` | 最终报告保留的聚合 defect 数。 |
| `comparison_window_seconds` | `3600` | 固定 UTC 时间窗口宽度。 |
| `min_template_members` | `3` | 少于三条请求不抽取模板。 |
| `stable_span_support_ratio` | `0.80` | 同一连续稳定片段所需支持率。 |
| `min_stable_support` | `3` | 稳定片段最小支持条数。 |
| `fixed_variant_max` | `3` | 变体数不超过该值且少于成员数时归为少数固定版本，否则内容持续变化。 |
| `min_blocked_stable_bytes` | `64` | 小于此值不产生优化机会（字节）。 |
| `min_exact_anchor_bytes` | `24` | 反事实恢复稳定内容所需的最小锚点（字节）。 |
| `text_similarity_threshold` | `0.80` | 候选文本的有界 shingle 相似度门槛。 |
| `template_compatibility_threshold` | `0.68` | 同一模板候选的结构兼容门槛。 |
| `max_dynamic_coverage_ratio` | `0.35` | 动态区域超过此比例时抑制不可靠模板。 |
| `max_candidates_per_request` | `128` | 每请求候选数上限。 |
| `max_projection_units` | `512` | 每请求投影单元数，超限跳过该请求。 |
| `max_text_unit_bytes` | `1048576` | 单文本单元上限，超限跳过该请求。 |
| `max_payload_bytes` | `8388608` | 单 payload 上限，超限跳过该请求。 |
| `max_alignment_cells` | `2000000` | 单次文本/单元对齐 cells 上限，超限跳过该对齐。 |
| `max_total_alignment_cells` | `64000000` | 单 cohort 累计对齐 cells 上限，超限跳过该 cohort。 |
| `max_records` | `1000000` | 单次分析记录数上限，超限拒绝继续加载。 |
| `prefix_view_max_nodes` | `5000` | 每模板前缀视图的节点上限；超出时视图截断，`0` 关闭 `templates[].prefix_view` 输出。 |
| `prefix_view_excerpt_bytes` | `96` | 前缀视图节点的文本摘录上限。 |

超限、坏行与未知 payload dialect 均在 `analysis.json` 中以 skipped 原因呈现；系统不会截断请求后输出高置信结论。

放开 `max_alignment_cells` / `max_total_alignment_cells` 时，单次 LCS 内存约为 cells × 8 字节，请按机器内存设置。

## 采集存储

Receiver、Analyzer 与 Export 可以读取同一份 `--storage-config` 配置。存储路径相对于配置文件所在目录解析。

SQLite 配置可保存为 `sqlite.json`。

```json
{
  "backend": "sqlite",
  "path": "captures.sqlite"
}
```

NDJSON 配置可保存为 `ndjson.json`。

```json
{
  "backend": "ndjson",
  "directory": "captures",
  "segment_seconds": 3600,
  "max_segment_bytes": 67108864
}
```

`segment_seconds` 和 `max_segment_bytes` 可省略，默认值如上。托管后端自行创建 NDJSON 目录，按 UTC 时间窗口及大小轮转。单条记录大于大小上限时独占一个分片。应用通过存储配置读取所有分片。

两个后端都可额外设置 `"retention": { "max_age_seconds": 604800, "cleanup_interval_seconds": 60 }`。`max_age_seconds` 是记录按采集时间计算的最长可读时间。读者立即排除已到期记录，写入端也拒绝已到期的记录。Receiver 按周期执行清理，默认每 60 秒检查。SQLite 分批删除到期记录，NDJSON 在分片整个时间窗口到期后删除文件。省略 `retention` 时保留全量。

```bash
actrail-kv-receiver --storage-config sqlite.json
actrail-kv-analyze --storage-config sqlite.json --output analysis.json \
  --session-key session-a --from 2026-11-23 --until 2026-11-24
actrail-kv-export --storage-config sqlite.json --output exported.ndjson \
  --session-key session-a --from 2026-11-23 --until 2026-11-24
```

`--session-key` 精确匹配现有采集记录的 session ID。`--from` 包含起点，`--until` 排除终点。两者支持 `YYYY-MM-DD`（UTC 午夜）或 RFC 3339 时间戳。筛选条件可独立使用，也可组合。Export 以 `--output` 指定的路径发布标准 NDJSON，可交给原有 `--input` 入口分析。

托管存储的 Analyzer 在扫描超过 `max_records` 时结束并报错。旧文件入口沿用既有的跳过记录报告。

## Report

```bash
actrail-kv-report \
  --input /var/lib/actrail-kv/analysis.json \
  --output /var/lib/actrail-kv/report.html
```

| CLI 参数 | 默认值 | 说明 |
|---|---:|---|
| `--input` | 无，必填 | analyzer 产出的 `analysis.json`。 |
| `--output` | 无，必填 | 报告路径或基础路径；不得覆盖 input。 |
| `--format` | 无，默认双份 | `html` 只生成 HTML，`md` 只生成 Markdown；不传时同时生成两份。 |
| `--locale` | `zh-CN` | 报告固定 UI 文案语言；首期支持 `zh-CN`、`en-US`。 |

默认双份模式下，`report.html`、`report.md` 或无后缀的 `report` 都会派生为
`report.html` + `report.md`。传 `--format` 时严格写入 `--output` 指定的路径。

report 无可调算法参数，不提供配置文件。报告不访问网络、不重新运行算法。HTML 证据经过 HTML 转义；
Markdown 使用纯 GFM，不嵌入 HTML、CSS、SVG 或 JavaScript。
locale 只影响报告固定文案、表头和说明；请求原文、技术标识及 analyzer insights 保持原样。

报告首屏的 90% 结构复用率目标是报告层的产品参数，写死在 `actrail-kv-report` 中，不作为配置项提供；调整目标只需改报告代码，不需要重新分析。

## 当前适配范围

分析器只支持 OpenAI-compatible chat payload：字符串 `model`、数组 `messages` 和可选 `tools`。不同 comparison group 绝不互相聚类。没有 tokenizer、模型 chat template 或真实 KV 命中数据时，`blocked_stable_bytes` 是可观察请求结构的代理指标，而不是 Token、KV miss 或金额收益。
