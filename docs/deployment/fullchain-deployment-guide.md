<!-- 本文件说明 actrail-kv 自身的部署和外部供给方接入方式。 -->
# actrail-kv 部署指南

## 组件边界

观测组件、代理网关和其他数据供给方可分别向 `actrail-kv-receiver` 上报。Receiver 定义统一的 [HTTP 接收协议](../reference/receiver/input.md)。供给方负责把采集结果转换成该协议，并按各自项目文档配置采集和上报。

## 构建和启动

从仓库根目录构建。

```bash
cargo build --release --workspace --bins
```

启动 Receiver。默认监听 `127.0.0.1:8080`，设置和 SQLite 数据库位于 `~/.actrail/actrail-kv/`。

```bash
target/release/actrail-kv-receiver
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
  http://127.0.0.1:8080/requests
```

Receiver 返回 HTTP `202` 表示成功记录请求。响应状态、元数据和可选用量字段见[接收协议](../reference/receiver/input.md)。

## 分析和报告

```bash
target/release/actrail-kv-analyze
target/release/actrail-kv-report
```

默认分析和报告产物位于设置目录。使用自定义设置时，给各命令传入同一个 `--config <path>`。分析筛选与报告格式见[配置参考](../configuration.md)。

## 复制到宿主项目

只构建命令时，复制 `Cargo.toml`、`Cargo.lock`、`LICENSE` 和整个 `crates/`，保留它们的相对位置。需要运行测试时一并复制 `tests/` 和 `examples/`。需要 README 和常用文档时带上 `README.md`、`examples/`、`docs/configuration.md` 以及 `docs/deployment/`、`docs/reference/`、`docs/concepts/`。

常用的排除项如下。路径以组件根目录为基准。

```text
.git/
.agents/
target/
dev/
run/
conf/
data/
captures/
settings.json
.settings.json.lock
storage.json
sqlite.json
ndjson.json
requests.ndjson
exported.ndjson
analysis.json
report.html
report.md
docs/integration/
docs/architectures/
```

`docs/integration/` 按供给方归档。需要某个案例时只复制对应子目录。复制工具支持 `!` 例外规则时，可以先用 `docs/integration/*` 排除各子项，再用 `!docs/integration/<provider>/` 纳入所需目录。`docs/architectures/` 保存实现设计，可整目录省略。

宿主采用自己的 Git 与开发规则时，可以省略 `.gitignore`、`.gitattributes` 和 `AGENTS.md`。排除自定义配置和数据路径时，按目录区分运行数据和 `examples/` 里的 NDJSON 语料。
