<!-- 本文件是当前可运行版本的配置参考；只记录已实现的参数和固定默认值。 -->
# 配置参考

## 配置方式

v0.1.0 没有配置文件、环境变量或远程配置中心。三个二进制只接受下列命令行参数；未列出的算法参数不可在运行时覆盖。这样一次 `analysis.json` 对应一组可追溯、固定的分析规则。

## Receiver

```bash
actrail-kv-receiver \
  --listen 127.0.0.1:8080 \
  --output /var/lib/actrail-kv/requests.ndjson
```

| 参数 | 默认值 | 说明 |
|---|---:|---|
| `--listen` | `127.0.0.1:8080` | HTTP 监听地址。默认仅 loopback；跨主机接入应在内网反向代理后显式配置。 |
| `--output` | 无，必填 | 接收语料的 NDJSON 路径。进程独占该文件，文件权限收紧为仅属主可读写。 |

Receiver 接受 `POST /requests`，body 必须是完整 JSON object。`X-Actrail-Endpoint-Key` 必填；`X-Actrail-Agent-Key`、`X-Actrail-Model-Deployment-Key`、`X-Actrail-KV-Namespace` 和 `X-Actrail-Source` 可选。前三项 metadata 提供后参与 comparison group，source 只记录采集来源。

## Analyze

```bash
actrail-kv-analyze \
  --input /var/lib/actrail-kv/requests.ndjson \
  --output /var/lib/actrail-kv/analysis.json \
  --top-k 20 \
  --comparison-window-seconds 3600
```

| 参数 | 默认值 | 说明 |
|---|---:|---|
| `--input` | 无，必填 | receiver 产出的 NDJSON 语料。 |
| `--output` | 无，必填 | 分析 JSON 的新路径；不得与 input 为同一文件或同一硬链接。 |
| `--top-k` | `20` | 最终报告保留的聚合 defect 数，必须大于零。 |
| `--comparison-window-seconds` | `3600` | 固定 UTC 时间窗口宽度；只有同一窗口内的请求才相互比较。 |

### 固定算法阈值

以下是当前版本编译进程序的默认值，不是 CLI 参数；分析结果会保存本次运行的完整参数快照。修改它们属于算法版本变更，必须补充设计记录与验收测试后再发布。

| 配置 | 值 | 作用 |
|---|---:|---|
| 最小模板成员数 | `3` | 少于三条请求不抽取模板。 |
| stable span 支持率 / 最小支持数 | `0.80` / `3` | 同一连续稳定片段须同时达到两项要求。 |
| 最小被阻断稳定字节 | `64` bytes | 小于此值不产生优化机会。 |
| 最小精确锚点 | `24` bytes | 反事实恢复稳定内容所需的最小锚点。 |
| 文本相似度阈值 | `0.80` | 候选文本的有界 shingle 相似度门槛。 |
| 模板兼容度 | `0.68` | 同一模板候选的结构兼容门槛。 |
| 最大动态覆盖率 | `0.35` | 动态区域超过此比例时抑制不可靠模板。 |

### 固定资源预算

| 配置 | 值 | 超限行为 |
|---|---:|---|
| 每请求候选数 | `128` | 有界候选召回。 |
| 每请求投影单元数 | `512` | 跳过该请求。 |
| 单文本单元 | `1 MiB` | 跳过该请求。 |
| 单 payload | `8 MiB` | 跳过该请求。 |
| 单次对齐 | `2,000,000` cells | 跳过该对齐。 |
| 单 cohort 累计对齐 | `64,000,000` cells | 跳过该 cohort 的分析。 |
| 单次分析记录数 | `1,000,000` | 拒绝继续加载。 |

超限、坏行与未知 payload dialect 均在 `analysis.json` 中以 skipped 原因呈现；系统不会截断请求后输出高置信结论。

## Report

```bash
actrail-kv-report \
  --input /var/lib/actrail-kv/analysis.json \
  --output /var/lib/actrail-kv/report.html
```

| 参数 | 默认值 | 说明 |
|---|---:|---|
| `--input` | 无，必填 | analyzer 产出的 `analysis.json`。 |
| `--output` | 无，必填 | 静态 HTML 的新路径；不得覆盖 input。 |

报告不访问网络、不重新运行算法。所有来自请求的证据都会 HTML 转义。

## 当前适配范围

分析器只支持 OpenAI-compatible chat payload：字符串 `model`、数组 `messages` 和可选 `tools`。不同 comparison group 绝不互相聚类。没有 tokenizer、模型 chat template 或真实 KV 命中数据时，`blocked_stable_bytes` 是可观察请求结构的代理指标，而不是 Token、KV miss 或金额收益。
