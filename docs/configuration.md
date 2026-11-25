<!-- 本文件是当前可运行版本的配置参考；只记录已实现的参数与默认值。 -->
# 配置参考

## 统一设置

Receiver、Analyzer、Export 和 Report 共用 `~/.actrail/actrail-kv/settings.json`，也可以给任一命令传入 `--config <path>` 选择另一份设置。每个命令读取自己的段。Receiver、Analyzer 和 Export 默认共用顶层的 `storage` 段。

- 缺少文件或字段时，命令将本次所需的默认值写回，保留已有值和其他命令的段。无效 JSON 或正在使用的配置字段无效时会报错，并保留原文件。
- 显式 CLI 参数优先于设置文件，并只对当前调用生效。文件内的相对路径从设置文件目录解析，CLI 路径从当前目录解析。
- 默认数据库与产物位于设置文件目录。Receiver 写入 `captures.sqlite`，Analyzer 生成 `analysis.json`，Export 生成 `exported.ndjson`，Report 生成 `report.html` 与 `report.md`。

例如，可以先写一份只包含需要调整的设置，其余字段由各命令补齐。

```json
{
  "receiver": {"listen": "127.0.0.1:8080"},
  "storage": {"backend": "sqlite"},
  "analyzer": {
    "output": "analysis.json",
    "options": {"top_k": 20},
    "selection": {"session_keys": [], "time_ranges": [], "last_n": null}
  },
  "export": {},
  "report": {"locale": "zh-CN"}
}
```

既有平铺 Receiver、Analyzer JSON 仍可显式通过 `--config` 读取。旧的 `--storage-config` 可显式覆盖共享存储段，文件结构仍为仅含 `storage` 的 JSON。

## Receiver

```bash
actrail-kv-receiver
```

| CLI 参数 | 默认值 | 说明 |
|---|---:|---|
| `--listen` | `127.0.0.1:8080` | HTTP 监听地址。默认仅 loopback；跨主机接入应在内网反向代理后显式配置。 |
| `--output` | 来自 `receiver.output`，缺省使用共享存储 | 单文件 NDJSON 路径。进程独占该文件，Unix 文件权限为 `0600`。 |
| `--storage-config` | 来自共享 `storage` | 兼容的独立存储文件，可显式覆盖。与 CLI 的 `--output` 互斥。 |
| `--config` | `~/.actrail/actrail-kv/settings.json` | 统一设置文件路径。 |
| `--max-payload-bytes` | 来自配置 | 请求体上限覆盖；默认 `8 MiB`。 |

`receiver` 段支持 `listen`、`max_payload_bytes` 和 `output`。`output` 为 `null` 时使用共享存储。

Receiver 接受 `POST /requests`，body 必须是完整 JSON object。`X-Actrail-Endpoint-Key` 必填；`X-Actrail-Agent-Key`、`X-Actrail-Model-Deployment-Key`、`X-Actrail-KV-Namespace` 和 `X-Actrail-Source` 可选。前三项 metadata 提供后参与 comparison group，source 只记录采集来源。

## Analyze

```bash
actrail-kv-analyze
```

| CLI 参数 | 默认值 | 说明 |
|---|---:|---|
| `--input` | 来自 `analyzer.input`，其次为 `receiver.output`，最后为共享存储 | 指定单文件 NDJSON 语料。 |
| `--storage-config` | 来自共享 `storage` | 兼容的独立存储文件，可显式覆盖。与 CLI 的 `--input` 互斥。 |
| `--session-key <key>` | 所有 session | 可重复传入，精确匹配任一指定的 session ID。 |
| `--time-range <FROM..UNTIL>` | 所有时间 | 可重复传入，取各时间段的并集。 |
| `--from`、`--until` | 对应方向无界 | 指定一个时间段，可只给一侧。与 `--time-range` 二选一。 |
| `--last-n <N>` | 所有匹配记录 | 在 session 和时间条件匹配后，保留采集时间最新的 N 条记录。 |
| `--output` | `analyzer.output`，默认 `analysis.json` | 分析 JSON 的目标路径，必须位于采集存储之外。 |
| `--config` | `~/.actrail/actrail-kv/settings.json` | 统一设置文件路径。 |
| `--top-k` | `20` | 最终报告保留的聚合 defect 数，必须大于零。 |
| `--comparison-window-seconds` | `3600` | 固定 UTC 时间窗口宽度；只有同一窗口内的请求才相互比较。 |

下面的命令将指定 session 和时间段内最新的 100 条记录送入分析，再生成报告。筛选可用于 `--storage-config` 或 `--input`。

```bash
actrail-kv-analyze --config settings.json \
  --session-key session-a --session-key session-b \
  --time-range 2026-11-20..2026-11-21 --time-range 2026-11-23..2026-11-25 \
  --last-n 100
actrail-kv-report --config settings.json
```

时间段包含起点并排除终点。时间边界支持 `YYYY-MM-DD`（UTC 午夜）和 RFC 3339 时间戳。省略 `--session-key`、时间段或 `--last-n` 时，各自匹配所有 session、所有时间或全部匹配记录。

可在 `analyzer.selection` 保存 `session_keys` 数组、`time_ranges` 数组和 `last_n`。每个时间段形如 `{"from":"2026-11-20","until":"2026-11-21"}`。CLI 显式筛选会覆盖对应的一组设置。没有保存筛选且未传 CLI 时分析全部语料。

### JSON 配置参数

算法阈值与资源预算存放于 `analyzer.options`。每次分析的生效参数会写入 `analysis.json` 的 `run.options`。

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

Receiver、Analyzer 与 Export 读取统一设置中的 `storage`。`storage.backend` 选择后端，`storage.backends` 保存各后端的参数。默认后端是 SQLite。下面的配置可合并到同一份 `settings.json`。

```json
{
  "storage": {
    "backend": "sqlite",
    "retention": {
      "max_age_seconds": 604800
    },
    "backends": {
      "sqlite": {
        "path": "captures.sqlite",
        "retention": {
          "max_age_seconds": 1209600,
          "delete_batch_size": 256
        }
      },
      "ndjson": {
        "directory": "captures",
        "segment_seconds": 3600,
        "max_segment_bytes": 67108864,
        "retention": {
          "cleanup_interval_seconds": 60,
          "max_segments_per_cleanup": 128
        }
      }
    }
  }
}
```

切换到 NDJSON 时，将 `storage.backend` 改为 `ndjson` 并重启 Receiver。`storage.backends.ndjson` 下的 `segment_seconds` 和 `max_segment_bytes` 可以省略，默认值如上。托管后端自行创建 NDJSON 目录，按 UTC 时间窗口及大小轮转。单条记录大于大小上限时独占一个分片。

`storage.retention.max_age_seconds` 定义公共的默认保留期限。每个后端可通过自己的 `retention.max_age_seconds` 覆盖。上例中 SQLite 保留 14 天，NDJSON 继承外层的 7 天。两层都省略期限时保留全量。

执行参数属于各后端的 `retention`。`cleanup_interval_seconds` 设置该后端的清理周期，默认 60 秒。SQLite 的 `delete_batch_size` 设置每批删除的行数，默认 256。NDJSON 的 `max_segments_per_cleanup` 限制每次检查回收的分片数，省略时回收全部到期分片。NDJSON 在分片的整个时间窗口到期后回收文件。两种后端的新读操作都会立即排除已到期的记录，写入端也会拒绝已到期的记录。

托管存储的 Analyzer 在扫描超过 `max_records` 时结束并报错。旧文件入口沿用既有的跳过记录报告。

## Export

```bash
actrail-kv-export --config settings.json \
  --session-key session-a --from 2026-11-23 --until 2026-11-24
```

`export` 段支持 `output`、`session_key`、`from`、`until`。默认从共享存储导出全部记录到配置目录的 `exported.ndjson`。导出的标准 NDJSON 可交给 Analyzer 的 `--input`。可显式传 `--storage-config` 覆盖导出的存储。

## Report

```bash
actrail-kv-report
```

| CLI 参数 | 默认值 | 说明 |
|---|---:|---|
| `--input` | `report.input`，缺省跟随 `analyzer.output` | Analyzer 产出的分析 JSON，最终回退到配置目录的 `analysis.json`。 |
| `--output` | `report.output`，缺省按格式命名 | 报告路径或基础路径。 |
| `--format` | 无，默认双份 | `html` 只生成 HTML，`md` 只生成 Markdown；不传时同时生成两份。 |
| `--locale` | `zh-CN` | 报告固定 UI 文案语言；首期支持 `zh-CN`、`en-US`。 |

默认双份模式下，`report.html`、`report.md` 或无后缀的 `report` 都会派生为
`report.html` + `report.md`。指定单一格式时，显式输出路径原样使用，默认生成 `report.html` 或 `report.md`。

`report` 段支持 `input`、`output`、`format` 和 `locale`。`input`、`output` 和 `format` 的自动补全值为 `null`，分别代表跟随分析产物、按格式命名和生成双份。HTML 证据经过 HTML 转义，Markdown 使用纯 GFM。
locale 只影响报告固定文案、表头和说明；请求原文、技术标识及 analyzer insights 保持原样。

报告首屏的 90% 结构复用率目标是报告层的产品参数，写死在 `actrail-kv-report` 中，不作为配置项提供；调整目标只需改报告代码，不需要重新分析。

## 当前适配范围

分析器只支持 OpenAI-compatible chat payload：字符串 `model`、数组 `messages` 和可选 `tools`。不同 comparison group 绝不互相聚类。没有 tokenizer、模型 chat template 或真实 KV 命中数据时，`blocked_stable_bytes` 是可观察请求结构的代理指标，而不是 Token、KV miss 或金额收益。
