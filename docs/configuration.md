<!-- 本文件是当前可运行版本的配置参考；只记录已实现的参数与默认值。 -->
# 配置参考

## 配置方式

三个二进制都接受命令行参数；analyze 与 receiver 额外支持 JSON 配置文件。

- 配置优先级：编译默认值 < JSON 配置文件 < 显式 CLI 参数。
- JSON 配置通过 `--config <path>` 传入；未提供时使用编译默认值，行为与旧版本一致。
- 配置包含未知字段或非法数值时拒绝启动。
- 每次分析仍会把实际生效参数完整写入 `analysis.json` 的 `run.options`，保证可追溯。

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
| `--output` | 无，必填 | 接收语料的 NDJSON 路径。进程独占该文件，文件权限收紧为仅属主可读写。 |
| `--config` | 无 | 可选 JSON 配置；CLI 显式参数覆盖文件。 |
| `--max-payload-bytes` | 来自配置 | 请求体上限覆盖；默认 `8 MiB`。 |

`receiver.json` 支持字段：`listen`、`max_payload_bytes`（默认 `8388608`）。示例见 `examples/receiver.config.example.json`。

Receiver 接受 `POST /requests`，body 必须是完整 JSON object。`X-Actrail-Endpoint-Key` 必填；`X-Actrail-Agent-Key`、`X-Actrail-Model-Deployment-Key`、`X-Actrail-KV-Namespace`、`X-Actrail-Source` 和 `X-Actrail-Session-Id` 可选。前三项 metadata 提供后参与 comparison group，source 只记录采集来源；session ID 只建立 Session 时间线，不进入模型可见内容或 comparison group。

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
| `--input` | 无，必填 | receiver 产出的 NDJSON 语料。 |
| `--output` | 无，必填 | 分析 JSON 的新路径；不得与 input 为同一文件或同一硬链接。 |
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

同一模板最多探测四个有序 episode：一个直接缺陷和最多三个条件性局部位点。该上限在当前版本中固定，不提供配置项。Session 时间线只比较同一 `session_id` 的相邻请求，也没有额外配置项。

超限、坏行与未知 payload dialect 均在 `analysis.json` 中以 skipped 原因呈现；系统不会截断请求后输出高置信结论。

放开 `max_alignment_cells` / `max_total_alignment_cells` 时，单次 LCS 内存约为 cells × 8 字节，请按机器内存设置。

## Report

```bash
actrail-kv-report \
  --input /var/lib/actrail-kv/analysis.json \
  --output /var/lib/actrail-kv/report.html
```

| CLI 参数 | 默认值 | 说明 |
|---|---:|---|
| `--input` | 无，必填 | analyzer 产出的 `analysis.json`。 |
| `--output` | 无，必填 | 静态 HTML 的新路径；不得覆盖 input。 |

report 无可调算法参数，不提供配置文件。报告不访问网络、不重新运行算法。所有来自请求的证据都会 HTML 转义。

## 当前适配范围

分析器只支持 OpenAI-compatible chat payload：字符串 `model`、数组 `messages` 和可选 `tools`。不同 comparison group 绝不互相聚类。没有 tokenizer、模型 chat template 或真实 KV 命中数据时，`blocked_stable_bytes` 是可观察请求结构的代理指标，而不是 Token、KV miss 或金额收益。
