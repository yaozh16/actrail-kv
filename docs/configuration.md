<!-- 本文件说明业务配置段和命令行参数的使用方式。 -->
# 配置参考

## 统一设置

Receiver、Analyzer、Export 和 Report 共用 `~/.actrail/actrail-kv/settings.toml`，也可以给任一命令传入 `--config <path>` 选择另一份设置。每个命令读取自己的段。Receiver、Analyzer 和 Export 默认共用顶层的 `storage` 段。

- 缺少文件或字段时，命令将本次所需的默认值写回，保留已有值和其他命令的段。无效 TOML 或正在使用的配置字段无效时会报错，并保留原文件。
- 显式 CLI 参数优先于设置文件，并只对当前调用生效。文件内的相对路径从设置文件目录解析，CLI 路径从当前目录解析。
- 默认数据库与产物位于设置文件目录。Receiver 写入 `captures.sqlite`，Analyzer 生成 `analysis.json`，Export 生成 `exported.ndjson`，Report 生成 `report.html` 与 `report.md`。

例如，可以先写一份只包含需要调整的设置，其余字段由各命令补齐。

```toml
[receiver]
listen = "127.0.0.1:8080"

[storage]
backend = "sqlite"

[analyzer]
output = "analysis.json"

[analyzer.options]
top_k = 20

[analyzer.selection]
session_keys = []
time_ranges = []

[export]

[report]
locale = "zh-CN"
```

`--storage-config` 可指定包含 `[storage]` 的独立 TOML 文件，覆盖本次命令使用的存储设置。

## Receiver

```bash
actrail-kv-receiver
```

| CLI 参数 | 默认值 | 说明 |
|---|---:|---|
| `--listen` | `127.0.0.1:8080` | HTTP 监听地址。默认仅 loopback，跨主机接入应在内网反向代理后显式配置 |
| `--output` | 来自 `receiver.output`，缺省使用共享存储 | 单文件 NDJSON 路径。进程独占该文件，Unix 文件权限为 `0600` |
| `--storage-config` | 来自共享 `storage` | 独立 TOML 存储配置文件，可显式覆盖。与 CLI 的 `--output` 互斥 |
| `--config` | `~/.actrail/actrail-kv/settings.toml` | 统一设置文件路径 |
| `--max-payload-bytes` | 来自配置 | 请求体上限覆盖，默认 `8 MiB` |

`receiver` 段支持 `listen`、`max_payload_bytes` 和 `output`。省略 `output` 时使用共享存储。

Receiver 接受 `POST /requests`，body 必须是完整 JSON object。`X-Actrail-Endpoint-Key` 必填，`X-Actrail-Agent-Key`、`X-Actrail-Model-Deployment-Key`、`X-Actrail-KV-Namespace` 和 `X-Actrail-Source` 可选。前三项 metadata 提供后参与 comparison group，source 只记录采集来源。

## Analyze

```bash
actrail-kv-analyze
```

| CLI 参数 | 默认值 | 说明 |
|---|---:|---|
| `--input` | 来自 `analyzer.input`，其次为 `receiver.output`，最后为共享存储 | 指定单文件 NDJSON 语料 |
| `--storage-config` | 来自共享 `storage` | 独立 TOML 存储配置文件，可显式覆盖。与 CLI 的 `--input` 互斥 |
| `--session-key <key>` | 所有 session | 可重复传入，精确匹配任一指定的 session ID |
| `--time-range <FROM..UNTIL>` | 所有时间 | 可重复传入，取各时间段的并集 |
| `--from`、`--until` | 对应方向无界 | 指定一个时间段，可只给一侧。与 `--time-range` 二选一 |
| `--last-n <N>` | 所有匹配记录 | 在 session 和时间条件匹配后，保留采集时间最新的 N 条记录 |
| `--output` | `analyzer.output`，默认 `analysis.json` | 分析 JSON 的目标路径，必须位于采集存储之外 |
| `--config` | `~/.actrail/actrail-kv/settings.toml` | 统一设置文件路径 |
| `--top-k` | `20` | 最终报告保留的聚合 defect 数，必须大于零 |
| `--comparison-window-seconds` | `3600` | 固定 UTC 时间窗口宽度，只有同一窗口内的请求才相互比较 |

下面的命令将指定 session 和时间段内最新的 100 条记录送入分析，再生成报告。筛选可用于 `--storage-config` 或 `--input`。

```bash
actrail-kv-analyze --config settings.toml \
  --session-key session-a --session-key session-b \
  --time-range 2026-11-20..2026-11-21 --time-range 2026-11-23..2026-11-25 \
  --last-n 100
actrail-kv-report --config settings.toml
```

时间段包含起点并排除终点。时间边界支持 `YYYY-MM-DD`（UTC 午夜）和 RFC 3339 时间戳。省略 `--session-key`、时间段或 `--last-n` 时，各自匹配所有 session、所有时间或全部匹配记录。

可在 `analyzer.selection` 保存 `session_keys` 数组、`time_ranges` 数组和 `last_n`。每个时间段使用 `[[analyzer.selection.time_ranges]]` 表，包含 `from = "2026-11-20"` 和 `until = "2026-11-21"` 等边界。CLI 显式筛选会覆盖对应的一组设置。没有保存筛选且未传 CLI 时分析全部语料。

### 分析配置参数

算法阈值与资源上限存放于 `analyzer.options` 的子表中。各子表直接对应共享契约中的子模块配置，`stability` 由模板提取和缺陷诊断共用。每次分析的生效参数会写入 `analysis.json` 的 `run.options`。

| 字段 | 默认值 | 作用 |
|---|---:|---|
| `top_k` | `20` | 最终报告保留的聚合 defect 数 |
| `projection.comparison_window_seconds` | `3600` | 固定 UTC 时间窗口宽度 |
| `candidate.min_members` | `3` | 少于三条请求不抽取模板 |
| `stability.stable_support_ratio` | `0.80` | 同一连续稳定片段所需支持率 |
| `stability.min_stable_support` | `3` | 稳定片段最小支持条数 |
| `diagnosis.fixed_variant_max` | `3` | 变体数不超过该值且少于成员数时归为少数固定版本，否则内容持续变化 |
| `diagnosis.min_blocked_bytes` | `64` | 小于此值不产生优化机会（字节） |
| `stability.min_anchor_bytes` | `24` | 反事实恢复稳定内容所需的最小锚点（字节） |
| `candidate.compatibility_threshold` | `0.68` | 同一模板候选的结构兼容门槛 |
| `candidate.max_dynamic_coverage_ratio` | `0.35` | 动态区域超过此比例时抑制不可靠模板 |
| `candidate.max_candidates_per_request` | `128` | 每请求候选数上限 |
| `projection.max_units` | `512` | 每请求投影单元数，超限跳过该请求 |
| `projection.max_text_unit_bytes` | `1048576` | 单文本单元上限，超限跳过该请求 |
| `corpus.max_record_bytes` | `8388608` | 单 payload 上限，超限跳过该请求 |
| `template.max_alignment_cells` | `2000000` | 单次文本/单元对齐 cells 上限，超限跳过该对齐 |
| `template.max_total_alignment_cells` | `64000000` | 单 cohort 累计对齐 cells 上限，超限跳过该 cohort |
| `corpus.max_records` | `1000000` | 单次分析记录数上限，超限拒绝继续加载 |
| `prefix_view.max_nodes` | `5000` | 每模板前缀视图的节点上限，超出时视图截断，`0` 关闭 `templates[].prefix_view` 输出 |
| `prefix_view.excerpt_bytes` | `96` | 前缀视图节点的文本摘录上限 |

超限、坏行与未知 payload dialect 均在 `analysis.json` 中以 skipped 原因呈现，系统不会截断请求后输出高置信结论。

放开 `template.max_alignment_cells` / `template.max_total_alignment_cells` 时，单次 LCS 内存约为 cells × 8 字节，请按机器内存设置。

## 采集存储

Receiver、Analyzer 与 Export 读取统一设置中的 `storage`。`storage.backend` 选择后端，`storage.backends` 保存各后端的参数。默认后端是 SQLite。下面的配置可合并到同一份 `settings.toml`。

```toml
[storage]
backend = "sqlite"

[storage.retention]
max_age_seconds = 604800

[storage.backends]

[storage.backends.sqlite]
path = "captures.sqlite"

[storage.backends.sqlite.retention]
max_age_seconds = 1209600
delete_batch_size = 256

[storage.backends.ndjson]
directory = "captures"
segment_seconds = 3600
max_segment_bytes = 67108864

[storage.backends.ndjson.retention]
cleanup_interval_seconds = 60
max_segments_per_cleanup = 128
```

切换到 NDJSON 时，将 `storage.backend` 改为 `ndjson` 并重启 Receiver。`storage.backends.ndjson` 下的 `segment_seconds` 和 `max_segment_bytes` 可以省略，默认值如上。托管后端自行创建 NDJSON 目录，按 UTC 时间窗口及大小轮转。单条记录大于大小上限时独占一个分片。

`storage.retention.max_age_seconds` 定义公共的默认保留期限。每个后端可通过自己的 `retention.max_age_seconds` 覆盖。上例中 SQLite 保留 14 天，NDJSON 继承外层的 7 天。共享保留时长默认 `0`，表示关闭过期清理。后端未设置保留时长时继承共享值，后端显式设为 `0` 时关闭该后端的过期清理。

执行参数属于各后端的 `retention`。`cleanup_interval_seconds` 设置该后端的清理周期，默认 60 秒。SQLite 的 `delete_batch_size` 设置每批删除的行数，默认 256。NDJSON 的 `max_segments_per_cleanup` 限制每次检查回收的分片数，省略时回收全部到期分片。NDJSON 在分片的整个时间窗口到期后回收文件。两种后端的新读操作都会立即排除已到期的记录，写入端也会拒绝已到期的记录。

托管存储的 Analyzer 在扫描超过 `corpus.max_records` 时结束并报错。旧文件入口沿用既有的跳过记录报告。

## Export

```bash
actrail-kv-export --config settings.toml \
  --session-key session-a --from 2026-11-23 --until 2026-11-24
```

`export` 段支持 `output`、`session_key`、`from`、`until`。默认从共享存储导出全部记录到配置目录的 `exported.ndjson`。导出的标准 NDJSON 可交给 Analyzer 的 `--input`。可显式传 `--storage-config` 覆盖导出的存储。

## Report

```bash
actrail-kv-report
```

| CLI 参数 | 默认值 | 说明 |
|---|---:|---|
| `--input` | `report.input`，缺省跟随 `analyzer.output` | Analyzer 产出的分析 JSON，最终回退到配置目录的 `analysis.json` |
| `--output` | `report.output`，缺省按格式命名 | 报告路径或基础路径 |
| `--format` | 无，默认双份 | `html` 只生成 HTML，`md` 只生成 Markdown，不传时同时生成两份 |
| `--locale` | `zh-CN` | 报告固定 UI 文案语言，首期支持 `zh-CN`、`en-US` |

默认双份模式下，`report.html`、`report.md` 或无后缀的 `report` 都会派生为
`report.html` + `report.md`。指定单一格式时，显式输出路径原样使用，默认生成 `report.html` 或 `report.md`。

`report` 段支持 `input`、`output`、`format` 和 `locale`。省略 `input`、`output` 和 `format` 时，分别跟随分析产物、按格式命名和生成双份。HTML 证据经过 HTML 转义，Markdown 使用纯 GFM。
locale 只影响报告固定文案、表头和说明，请求原文、技术标识及 analyzer insights 保持原样。

报告首屏的 90% 结构复用率目标是报告层的产品参数，写死在 `actrail-kv-report` 中，不作为配置项提供，调整目标只需改报告代码，不需要重新分析。

## 当前适配范围

分析器只支持 OpenAI-compatible chat payload：字符串 `model`、数组 `messages` 和可选 `tools`。不同 comparison group 绝不互相聚类。没有 tokenizer、模型 chat template 或真实 KV 命中数据时，`blocked_stable_bytes` 是可观察请求结构的代理指标，而不是 Token、KV miss 或金额收益。

## 生成示例配置

示例文件由业务配置的默认值生成，字段注释来自 Rust 文档注释。修改配置类型后，在仓库根目录重新生成：

```bash
cargo run -q -p actrail-kv-analyze --example generate_config \
  > examples/analyze.config.example.toml
cargo run -q -p actrail-kv-receiver-bin --example generate_config \
  > examples/receiver.config.example.toml
```

## 未知配置字段

配置中出现当前版本不认识的字段时，命令输出包含字段路径的 warning，并继续读取已知字段。检查范围仅限本次读取的业务配置段。补齐默认值时，已有字段和注释会保留，包括未知字段。字段类型错误或 TOML 语法错误仍会导致加载失败。
