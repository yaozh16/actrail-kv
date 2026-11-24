<!-- 本文件说明 actrail-kv 自身的部署和外部供给方接入方式。 -->
# actrail-kv 部署指南

## 组件边界

观测组件、代理网关和其他数据供给方可分别向 `actrail-kv-receiver` 上报。Receiver 定义统一的 [HTTP 接收协议](../architectures/receiver/input.md)。供给方负责把采集结果转换成该协议，并按各自项目文档配置采集和上报。

## 构建和启动

从仓库根目录构建。

```bash
cargo build --release --workspace --bins
```

启动 Receiver 并指定该实例的监听地址。示例使用单文件输出。

```bash
target/release/actrail-kv-receiver --listen 127.0.0.1:8087 --output ./requests.ndjson
```

Windows 的可执行文件位于 `target\release\`，文件扩展名为 `.exe`。Receiver 运行参数和输出配置见[配置参考](../configuration.md#receiver)。

## 验证接收

向 `POST /requests` 发送完整模型请求。上报时必填 `X-Actrail-Endpoint-Key`，并可通过 `X-Actrail-Source` 和 `X-Actrail-Session-Key` 标识来源和会话。

```bash
curl --fail-with-body \
  -H 'Content-Type: application/json' \
  -H 'X-Actrail-Endpoint-Key: llm-primary' \
  -H 'X-Actrail-Source: smoke-test' \
  --data '{"model":"example","messages":[{"role":"user","content":"hello"}]}' \
  http://127.0.0.1:8087/requests
```

Receiver 返回 HTTP `202` 表示成功记录请求。响应状态、元数据和可选用量字段见[接收协议](../architectures/receiver/input.md)。

## 分析和报告

```bash
target/release/actrail-kv-analyze --input ./requests.ndjson --output ./analysis.json
target/release/actrail-kv-report --input ./analysis.json --output ./report.html
```

分析筛选与报告格式见[配置参考](../configuration.md)。
