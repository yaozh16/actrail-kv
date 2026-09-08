<!-- 本文件说明 agent-cassette 接入、协议转换与三二进制全链路部署操作。 -->
# agent 接入 agent-cassette 并部署 actrail-kv 全链路

## 1. 范围与拓扑

本文描述如何部署并运行以下链路：

```text
agent（OpenAI 兼容 或 Anthropic Messages 协议）
  → agent-cassette / 上游插件（录制 + 转发；向 actrail 直推前完成 Anthropic → OAI 转换）
  → actrail-kv-receiver（OAI 请求）→ ndjson
  → actrail-kv-analyze → analysis.json
  → actrail-kv-report → report.html
```

组件职责：

| 组件 | 职责 |
|---|---|
| agent-cassette / 上游插件 | 接收 OpenAI `/v1/chat/completions` 与 Anthropic `/v1/messages`；录制、转发至真实 LLM 后端；向 actrail receiver 直推前，将 Anthropic 请求转换为 OAI 格式 |
| actrail-kv-receiver | 接收直推请求并追加写入 ndjson |
| actrail-kv-analyze | 从 ndjson 聚类模板、识别结构缺陷、输出 `analysis.json` |
| actrail-kv-report | 将 `analysis.json` 渲染为静态 HTML 报告 |

协议转换由上游完成；Receiver 落盘及 Analyzer 投影使用转换后的 OAI 请求，当前没有原生 Anthropic 分析适配器。

## 2. 部署前准备

### 2.1 目录与仓库

两个仓库放置在同一级目录：

```text
<root>/
├── agent-cassette/
└── actrail-kv/
```

### 2.2 编译环境

Linux：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustc --version
cargo --version
```

Windows（PowerShell）：

```powershell
rustup show active-toolchain
rustup target add x86_64-pc-windows-msvc
```

Windows 需要 Visual Studio Build Tools 2022，工作负载“使用 C++ 的桌面开发”（MSVC v143 与 Windows SDK）。
agent-cassette 依赖 ring / aws-lc 原生 C 库，缺少该工具链时编译会失败。

### 2.3 上游后端与密钥

agent-cassette 通过 `[proxy.provider]` 指向真实 LLM 后端。需要准备：

- OpenAI 兼容后端：`api_base` 形如 `https://host/v1`
- Anthropic 兼容后端：`api_base` 形如 `https://host`
- 模型 id（如 `GLM-5.3`）
- 密钥环境变量（如 `DASHSCOPE_API_KEY`），由配置中的 `api_key_env` 指定

## 3. Linux 部署

### 3.1 拉取仓库并切换分支

```bash
mkdir -p ~/actrail && cd ~/actrail

git clone git@gitcode.com:yaozhenhe/agent-cassette.git
git clone git@gitcode.com:yaozhenhe/actrail-kv.git

cd agent-cassette
git checkout dev
git pull --ff-only

cd ../actrail-kv
git checkout dev
git pull --ff-only
```

### 3.2 编译

```bash
cd ~/actrail/agent-cassette
cargo build --release -p agent-cassette

cd ~/actrail/actrail-kv
cargo build --release --workspace --bins
```

产物：

```text
agent-cassette/target/release/agent-cassette
actrail-kv/target/release/actrail-kv-receiver
actrail-kv/target/release/actrail-kv-analyze
actrail-kv/target/release/actrail-kv-report
```

### 3.3 配置 agent-cassette

从模板生成配置：

```bash
mkdir -p ~/actrail/conf
cp ~/actrail/agent-cassette/config.example.toml ~/actrail/conf/agent-cassette.toml
```

编辑关键段：

```toml
[gateway.router]
default_mode = "forward"

[gateway.transport]
listen_addr = "127.0.0.1:18082"

[proxy.forward]
recording = true

[proxy.provider]
provider = "dashscope"             # dashscope / openai / anthropic
model = "GLM-5.3"
api_base = "http://<backend>:8080/v1"   # OpenAI 兼容协议需保留 /v1
api_key_env = "DASHSCOPE_API_KEY"

[recorder.session]
retain_request_raw = true

[recorder.actrail]
enabled = true
receiver_url = "http://127.0.0.1:8087"
endpoint_key = "dashscope-primary"
agent_key = "agent-a"
model_deployment_key = "GLM-5.3"
session_source = "recorder"
source = "agent-cassette"
```

导出密钥：

```bash
export DASHSCOPE_API_KEY="..."
```

### 3.4 启动 receiver 与 agent-cassette

```bash
mkdir -p ~/actrail/run

cd ~/actrail/actrail-kv
nohup target/release/actrail-kv-receiver \
  --listen 127.0.0.1:8087 \
  --output ~/actrail/run/requests.ndjson \
  > ~/actrail/run/receiver.log 2>&1 &

cd ~/actrail/agent-cassette
nohup target/release/agent-cassette \
  -c ~/actrail/conf/agent-cassette.toml start \
  > ~/actrail/run/cassette.log 2>&1 &
```

验证：

```bash
curl -fsS http://127.0.0.1:18082/healthz
```

### 3.5 agent 接入配置

Anthropic Messages 协议：

```bash
export ANTHROPIC_BASE_URL="http://127.0.0.1:18082"
export ANTHROPIC_API_KEY="cassette-proxy"
export ANTHROPIC_MODEL="GLM-5.3"
```

OpenAI 兼容协议：

```bash
export OPENAI_BASE_URL="http://127.0.0.1:18082/v1"
export OPENAI_API_KEY="cassette-proxy"
export OPENAI_MODEL="GLM-5.3"
```

随后启动 agent 并执行任务，receiver 会持续追加 ndjson。

### 3.6 分析并生成报告

```bash
# 生成 analyze 配置：复制仓库自带示例；长 system prompt 时可在该文件中
# 上调 max_alignment_cells / max_total_alignment_cells，避免模板被对齐预算跳过
mkdir -p ~/actrail/run
cp ~/actrail/actrail-kv/examples/analyze.config.example.json \
   ~/actrail/run/analyze-open-budgets.json

REPORT_DIR=~/actrail/reports/run-$(date +%Y%m%d-%H%M%S)
mkdir -p "$REPORT_DIR"

cd ~/actrail/actrail-kv
target/release/actrail-kv-analyze \
  --config ~/actrail/run/analyze-open-budgets.json \
  --input ~/actrail/run/requests.ndjson \
  --output "$REPORT_DIR/analysis.json"

target/release/actrail-kv-report \
  --input "$REPORT_DIR/analysis.json" \
  --output "$REPORT_DIR/report.html"
```

`report.html` 为自包含静态文件，可直接用浏览器打开，或置于静态目录（如 nginx）供访问。

## 4. Windows 部署

### 4.1 拉取与编译

```powershell
mkdir C:\actrail; cd C:\actrail
git clone https://gitcode.com/yaozhenhe/agent-cassette.git
git clone https://gitcode.com/yaozhenhe/actrail-kv.git

cd C:\actrail\agent-cassette
git checkout dev
git pull --ff-only
cargo build --release -p agent-cassette

cd C:\actrail\actrail-kv
git checkout dev
git pull --ff-only
cargo build --release --workspace --bins
```

产物为同目录下的 `.exe`。

### 4.2 配置

```powershell
New-Item -ItemType Directory -Force C:\actrail\conf | Out-Null
Copy-Item C:\actrail\agent-cassette\config.example.toml C:\actrail\conf\agent-cassette.toml
```

按 3.3 编辑配置，**必须修改以下字段**（模板中有 `CHANGE_ME` 占位符，部分字段为注释状态需取消注释）：

| 字段 | 默认值 | 修改为 | 说明 |
|---|---|---|---|
| `[gateway.router].default_mode` | `"replay"` | `"forward"` | 路由模式改为转发 |
| `[gateway.transport].listen_addr` | `"127.0.0.1:8080"` | `"127.0.0.1:18082"` | cassette 监听地址 |
| `[proxy.forward].recording` | `false` | `true` | 启用录制 |
| `[proxy.provider].api_base` | `"CHANGE_ME"` | `"http://host:8080/v1"` | 真实 LLM 后端地址 |
| `[proxy.provider].api_key_env` | `"CHANGE_ME"` | `"DASHSCOPE_API_KEY"` | 密钥环境变量名 |
| `[proxy.provider].model` | `"CHANGE_ME"` | `"GLM-5.3"` | 模型 id |
| `[recorder.actrail].enabled` | `false` | `true` | 启用 actrail 直推 |
| `[recorder.actrail].agent_key` | 注释状态 | `"agent-a"` | 取消注释并设置 agent 标识 |
| `[recorder.actrail].endpoint_key` | `"llm-primary"` | `"dashscope-primary"` | 端点标识 |
| `[recorder.actrail].model_deployment_key` | 注释状态 | `"GLM-5.3"` | 取消注释并设置模型部署标识 |
| `[recorder.actrail].receiver_url` | `""` | `"http://127.0.0.1:8087"` | receiver 地址 |
| `[recorder.actrail].session_source` | `"none"` | `"recorder"` | 会话来源改为录制 |
| `[recorder.session].retain_request_raw` | `false` | `true` | 保留原始请求内容 |

设置密钥环境变量：

```powershell
$env:DASHSCOPE_API_KEY = "..."
```

### 4.3 启动

> **注意**：
> - **上游模型缓存**：cassette 首次启动时会将 config.toml 中 `[proxy.provider]` 的地址注册到 `data/upstreams.json`。后续重启时以此文件为准，改 config.toml 不会生效。若需更换 `api_base`，删除 `data/upstreams.json` 后重启即可重新注册。

```powershell
New-Item -ItemType Directory -Force C:\actrail\run | Out-Null

Start-Process -FilePath C:\actrail\actrail-kv\target\release\actrail-kv-receiver.exe `
  -ArgumentList @('--listen','127.0.0.1:8087','--output','C:\actrail\run\requests.ndjson') `
  -RedirectStandardOutput C:\actrail\run\receiver.log `
  -RedirectStandardError C:\actrail\run\receiver_err.log -WindowStyle Hidden

Start-Process -FilePath C:\actrail\agent-cassette\target\release\agent-cassette.exe `
  -ArgumentList @('-c','C:\actrail\conf\agent-cassette.toml','start') `
  -RedirectStandardOutput C:\actrail\run\cassette.log `
  -RedirectStandardError C:\actrail\run\cassette_err.log -WindowStyle Hidden

# 等待启动并验证
Start-Sleep -Seconds 3
Write-Host "=== 验证 cassette (端口 18082) ==="
Invoke-WebRequest -UseBasicParsing http://127.0.0.1:18082/healthz -TimeoutSec 5
Write-Host "=== 验证 receiver (端口 8087) ==="
# receiver 仅接受 POST /requests，尝试连接验证端口是否已监听
# 注意：Wait(2000) 超时返回 $false 而不抛异常；连接被拒绝时才抛异常，两种情况都要处理
$tcp = [System.Net.Sockets.TcpClient]::new()
try { $ok = $tcp.ConnectAsync('127.0.0.1', 8087).Wait(2000) } catch { $ok = $false }
if ($ok) { Write-Host "receiver 端口 8087 已监听" }
else { Write-Host "receiver 端口 8087 未监听，请检查 C:\actrail\run\receiver_err.log" }
$tcp.Dispose()
```

若 `cassette` 启动失败，检查 `C:\actrail\run\cassette_err.log` 中的错误信息，常见原因：
- 上游服务不可达 — 确认 `api_base` 地址正确且网络可达
- 密钥未设置 — 确认 `$env:DASHSCOPE_API_KEY` 已配置

### 4.4 agent 接入配置

Anthropic Messages 协议：

```powershell
$env:ANTHROPIC_BASE_URL = 'http://127.0.0.1:18082'
$env:ANTHROPIC_API_KEY   = 'cassette-proxy'
$env:ANTHROPIC_MODEL     = 'GLM-5.3'
```

OpenAI 兼容协议：

```powershell
$env:OPENAI_BASE_URL = 'http://127.0.0.1:18082/v1'
$env:OPENAI_API_KEY   = 'cassette-proxy'
$env:OPENAI_MODEL     = 'GLM-5.3'
```

随后启动 agent 并执行任务，receiver 会持续追加 ndjson。

### 4.5 分析并生成报告

```powershell
# 生成 analyze 配置：复制仓库自带示例；长 system prompt 时可上调
# max_alignment_cells / max_total_alignment_cells
New-Item -ItemType Directory -Force C:\actrail\run | Out-Null
Copy-Item C:\actrail\actrail-kv\examples\analyze.config.example.json `
          C:\actrail\run\analyze-open-budgets.json

$ReportDir = Join-Path 'C:\actrail\reports' ('run-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
New-Item -ItemType Directory -Force -Path $ReportDir | Out-Null

cd C:\actrail\actrail-kv
& .\target\release\actrail-kv-analyze.exe `
  --config C:\actrail\run\analyze-open-budgets.json `
  --input C:\actrail\run\requests.ndjson `
  --output (Join-Path $ReportDir 'analysis.json')

& .\target\release\actrail-kv-report.exe `
  --input (Join-Path $ReportDir 'analysis.json') `
  --output (Join-Path $ReportDir 'report.html')
```

## 5. 一键脚本

仓库 `docs/deployment/` 下提供两个管理脚本，覆盖 init / env / health / report / stop：

| 平台 | 文件 | 运行方式 |
|---|---|---|
| Linux | `fullchain-deploy-linux.sh` | `bash fullchain-deploy-linux.sh <command>` |
| Windows | `fullchain-deploy-windows.ps1` | `.\fullchain-deploy-windows.ps1 <command>` |

Linux 入口会加载同目录下的 `fullchain-deploy-linux/report.sh`。复制或分发脚本时需一并保留该子目录及相对布局。

命令：

| 命令 | 行为 |
|---|---|
| `init` | 端口空闲则启动 receiver 与 agent-cassette；被占用则复用；打印 agent 接入环境变量 |
| `env` | 输出 agent 所需 `ANTHROPIC_*` 环境变量 |
| `health` | 校验 healthz、receiver 端口、OpenAI 与 Anthropic 端点 |
| `report` | 对已有 ndjson 执行 analyze + report |
| `stop` | 停止由脚本启动的进程 |

脚本默认在 `actrail-kv/docs/deployment` 的上级查找同级 `agent-cassette` 仓库；
可用环境变量覆盖 `KV_ROOT`、`CASSETTE_REPO`、`ACTRAIL_REPO`、`CASSETTE_CONF`、
`RECEIVER_OUT`、`DOC_ROOT`。

`DOC_ROOT` 默认是 `$KV_ROOT/doc`（Windows 为对应的 `doc` 子目录）。已有报告服务若使用其他目录，需显式设置 `DOC_ROOT`。

Linux 构建优先使用当前 `PATH` 中的 Cargo，并继承已有 `RUSTUP_HOME`、`CARGO_HOME`。只有 PATH 中没有 Cargo 时，才查找 `CARGO_HOME/bin`；未设置 `CARGO_HOME` 时查找当前用户的 `$HOME/.cargo/bin`。可通过 `CARGO_BIN` 显式指定优先使用的可执行文件目录。

Linux：

```bash
cd ~/actrail/actrail-kv/docs/deployment
export KV_ROOT=~/actrail
export CASSETTE_CONF=~/actrail/conf/agent-cassette.toml

bash fullchain-deploy-linux.sh init
eval "$(bash fullchain-deploy-linux.sh env)"
# 启动 agent 执行任务
bash fullchain-deploy-linux.sh health
bash fullchain-deploy-linux.sh report
bash fullchain-deploy-linux.sh stop
```

Windows：

```powershell
cd C:\actrail\actrail-kv\docs\deployment
$env:KV_ROOT = 'C:\actrail'
$env:CASSETTE_CONF = 'C:\actrail\conf\agent-cassette.toml'
$env:DASHSCOPE_API_KEY = '...'   # init 前必须设置

.\fullchain-deploy-windows.ps1 init
iex (& '.\fullchain-deploy-windows.ps1' env)
.\fullchain-deploy-windows.ps1 health
.\fullchain-deploy-windows.ps1 report
.\fullchain-deploy-windows.ps1 stop
```

## 6. 数据验证

采集完成后核对：

```bash
wc -l ~/actrail/run/requests.ndjson
```

`analysis.json` 中可检查：

- `run.analyzed_records`：被分析请求数
- `templates`：聚类出的模板
- `defects`：结构缺陷
- `session_analysis.timelines`：Session 时间线与相邻请求的前缀变化证据；数组长度为时间线数量
- `session_analysis.history_sites`：聚合后的历史变化位置

模板与缺陷的关系：

- 模板在 ≥3 条相似请求（共享稳定 system/tool 前缀）时产生；
- 结构缺陷要求“同一位置的受控小变体 + 被阻断的稳定尾”；
- 仅有模板而无数值变化时，`defects` 为空属正常结果。

## 7. 常见故障

| 现象 | 处理 |
|---|---|
| Anthropic agent 请求返回 404 | agent-cassette 二进制未包含 `/v1/messages`；切换至含 `49e68c4` 的提交后重编 |
| OpenAI agent 请求返回 404 | `base_url` 需要包含 `/v1` |
| receiver 无数据 | 检查 `[recorder.actrail] enabled=true` 与 `receiver_url` 端口；查看 cassette 日志 |
| 分析报 0 模板 | 语料缺少同模板重复；使用同 system 的相似任务 × 多次或同会话多轮 |
| Windows 编译失败于 C 依赖 | 安装 VS Build Tools 并勾选 MSVC 与 Windows SDK |
| Windows 无守护模式 | Windows 平台不支持 `-d`；使用脚本 `init`/`stop` 管理 |
| cassette 启动后仍请求旧地址（如 `127.0.0.1:9999`） | 更换 `api_base` 后，`data/upstreams.json` 缓存了旧地址。删除该文件后重启 cassette 即可重新发现模型 |
| 首次编译极慢 | 国内网络建议配置 Rust 镜像源（如 TUNA `sparse+https://mirrors.tuna.tsinghua.edu.cn/crates.io-index/`），写入 `~/.cargo/config.toml` |

## 8. 相关文件

- 配置模板：`agent-cassette/config.example.toml`
- 分析配置示例：`actrail-kv/examples/analyze.config.example.json`
- 一键脚本：`docs/deployment/fullchain-deploy-linux.sh`、
  `docs/deployment/fullchain-deploy-windows.ps1`
- 报告说明：`actrail-kv/docs/configuration.md`、`actrail-kv/docs/concepts/`
