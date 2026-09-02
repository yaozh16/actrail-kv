<!-- 本文件说明三个二进制的构建、启动、调用、产物和当前分析边界。 -->
# Actrail KV

Actrail KV 离线分析完整 LLM HTTP 请求，抽取请求模板，定位本可复用但被较早差异阻断的稳定上下文，并生成聚合 Top K 报告。它不依赖 tokenizer、模型服务或真实 KV 命中遥测。

## 构建

```bash
cargo build --release --workspace --bins
```

生成三个二进制：

- `target/release/actrail-kv-receiver`
- `target/release/actrail-kv-analyze`
- `target/release/actrail-kv-report`

## 1. 接收完整请求 payload

```bash
target/release/actrail-kv-receiver \
  --listen 127.0.0.1:8080 \
  --output requests.ndjson
```

网关、插桩组件或 Agent hook 直接把发往模型 API 的完整 JSON body 提交过来：

```bash
curl --fail-with-body \
  -H 'content-type: application/json' \
  -H 'x-actrail-source: agent-instrumentation' \
  --data-binary @request.json \
  http://127.0.0.1:8080/requests
```

成功返回 HTTP `202`。`X-Actrail-Source` 可省略，只用于记录采集来源，不参与模板规则。

## 2. 离线分析

```bash
target/release/actrail-kv-analyze \
  --input requests.ndjson \
  --output analysis.json \
  --top-k 20
```

分析过程完全离线。未知 payload 结构、坏行和资源超限会在 `analysis.json` 中显式记录，不会伪装成已完成的高置信结果。

## 3. 生成报告

```bash
target/release/actrail-kv-report \
  --input analysis.json \
  --output report.html
```

报告是无脚本的静态 HTML。请求证据经过 HTML 转义，可以直接复制到隔离内网查看。

## MVP 支持边界

- 当前 adapter 支持 OpenAI-compatible chat payload：必须包含字符串 `model` 和数组 `messages`，可包含 `tools`。
- 不同 model 或 payload dialect 不会互相聚类；未知 dialect 显式跳过。
- Top K 的 `blocked_stable_bytes`、`confidence` 和 `score` 是可观察 UTF-8/结构代理，不等于真实 Token 数、KV miss 或金额收益。
- 优化建议是诊断反事实，不会自动改写请求；涉及内容重排时必须由使用方确认语义安全。

完整的命令行参数、固定算法阈值、资源预算及部署配置见[配置参考](docs/configuration.md)。

## 安全部署前提

- receiver 默认只监听 loopback。需要跨主机接入时，应由内网反向代理提供 TLS、客户端认证、访问控制和请求超时，再显式监听非 loopback 地址。
- 语料文件会被收紧为仅属主可读写，并由单个 receiver 独占锁定；仍应为所在磁盘设置配额和监控。需要切换语料文件时，优雅停止 receiver 后以新输出路径重新启动。
- 完整 payload 可能包含敏感数据；analysis 和 HTML 报告应沿用客户内网的数据分级、保留和销毁规则。

## 发布验收

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo test --release --workspace --all-targets
tests/end_to_end/three_binaries.sh
```
