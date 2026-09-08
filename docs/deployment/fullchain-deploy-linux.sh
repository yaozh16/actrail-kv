#!/usr/bin/env bash
# Full-chain deploy/manager for agent-cassette + actrail-kv (Linux one-click).
# Kept inside actrail-kv/docs/deployment; defaults assume sibling agent-cassette repo.
#
# 职责边界：
#   --init   启动/复用录制链（receiver + agent-cassette），输出 agent 接入所需环境变量
#   --env    输出 agent 接入所需的 ANTHROPIC_* 环境变量行（当前终端 eval 后启动 agent）
#   --report 用 receiver ndjson 生成 actrail-kv 报告
#   --stop   停止本脚本启动的录制链进程
#
# 脚本不启动 agent、不改任何代码、不改 agent-cassette 配置（只读复用 CASSETTE_CONF）。
set -euo pipefail

# ============================ 可覆盖配置区 ============================
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ -z "${KV_ROOT:-}" ]; then
  if ! KV_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"; then
    echo "无法定位部署根目录，请设置 KV_ROOT" >&2
    exit 1
  fi
fi
CASSETTE_REPO="${CASSETTE_REPO:-$KV_ROOT/agent-cassette}"
ACTRAIL_REPO="${ACTRAIL_REPO:-$KV_ROOT/actrail-kv}"
DOC_ROOT="${DOC_ROOT:-$KV_ROOT/doc}"

# 已配好的 agent-cassette 配置（真实上游 + recorder.actrail 均已配好；脚本只读）
CASSETTE_CONF="${CASSETTE_CONF:-$KV_ROOT/conf/agent-cassette.toml}"

RUN_NAME="${RUN_NAME:-anthropic-agent-$(date +%Y%m%d-%H%M%S)}"
STATE_FILE="$KV_ROOT/run/.anthropic-agent-current.env"

RECEIVER_OUT="${RECEIVER_OUT:-$KV_ROOT/run/requests-$RUN_NAME.ndjson}"
RECEIVER_LOG="$KV_ROOT/run/receiver-$RUN_NAME.log"
CASSETTE_LOG="$KV_ROOT/run/agent-cassette-$RUN_NAME.log"
OUT_DIR="$DOC_ROOT/actrail-kv-run-anthropic-agent-report-$(date +%Y%m%d-%H%M%S)"

# 尊重当前工具链环境；仅在 PATH 缺少 cargo 时补充标准安装目录。
if [ -n "${CARGO_BIN:-}" ]; then
  export PATH="$CARGO_BIN:$PATH"
elif ! command -v cargo >/dev/null 2>&1; then
  if [ -n "${CARGO_HOME:-}" ] && [ -x "$CARGO_HOME/bin/cargo" ]; then
    export PATH="$CARGO_HOME/bin:$PATH"
  elif [ -z "${CARGO_HOME:-}" ] && [ -n "${HOME:-}" ] && [ -x "$HOME/.cargo/bin/cargo" ]; then
    export PATH="$HOME/.cargo/bin:$PATH"
  fi
fi

SKIP_BUILD="${SKIP_BUILD:-0}"
FORCE_BUILD="${FORCE_BUILD:-0}"
PORT_CHECK="${PORT_CHECK:-false}"  # 默认不检查端口有效性（true 时端口占用直接报错）

if [ -n "${BUDGET_CONF:-}" ]; then
  ANALYZE_CONF="$BUDGET_CONF"
elif [ -f "$ACTRAIL_REPO/examples/analyze.config.example.json" ]; then
  ANALYZE_CONF="$ACTRAIL_REPO/examples/analyze.config.example.json"
else
  ANALYZE_CONF=""
fi

# ============================ 内部函数 ============================
log()  { printf '[%s] %s\n' "$(date +%H:%M:%S)" "$*"; }
fail() { log "FAIL: $*"; exit 1; }
ok()   { log "PASS: $*"; }

usage() {
  cat <<EOF
用法: bash fullchain-deploy-linux.sh <命令>

命令（脚本只管理录制链，不启动 agent）:
  --init | init     启动/复用录制链（receiver + agent-cassette）并打印 agent 接入 env
  --env | env       输出 ANTHROPIC_* 环境变量行；在当前终端 eval 后启动 agent
  --health | health 检查 healthz / receiver 端口 / OpenAI /v1/chat/completions / Anthropic /v1/messages
  --report | report 用已有 ndjson 运行 actrail-kv analyze/report
  --stop | stop     停止本脚本启动的录制链进程
  -h | --help       帮助

接入参数（cassette 已配好时一般不用传）:
  CASSETTE_CONF      已配好的 agent-cassette 配置路径（默认 \$KV_ROOT/conf/agent-cassette.toml）
  KV_ROOT / DOC_ROOT 部署根目录 / 报告目录（默认根据脚本位置定位 / \$KV_ROOT/doc）
  CARGO_BIN         可选 Cargo 可执行文件目录；未设置时优先使用当前 PATH
  ANTHROPIC_API_KEY（回退 ANTHROPIC_AUTH_TOKEN）/ ANTHROPIC_MODEL
                     --env 原样透传到启动 agent 的终端（可省）
EOF
  exit "${1:-0}"
}

# conf_get <section> <key> —— 极简 TOML 取值
conf_get() {
  python3 - "$CASSETTE_CONF" "$1" "$2" <<'PYEOF'
import re, sys
path, section, key = sys.argv[1], sys.argv[2], sys.argv[3]
cur = None
for raw in open(path, encoding="utf-8"):
    line = raw.strip()
    if line.startswith("[") and line.endswith("]"):
        cur = line[1:-1].strip()
        continue
    if cur != section:
        continue
    m = re.match(r'^' + re.escape(key) + r'\s*=\s*(.*)$', line)
    if m:
        val = m.group(1).strip()
        print(val.strip('"').strip("'"))
        break
PYEOF
}

port_of() {
  python3 - "$1" <<'PYEOF'
import re, sys
m = re.search(r':(\d+)\s*$', sys.argv[1])
print(m.group(1) if m else "")
PYEOF
}

wait_port() {
  local port="$1" tries=0
  while ! (exec 3<>"/dev/tcp/127.0.0.1/$port") 2>/dev/null; do
    tries=$((tries + 1))
    [ "$tries" -le 60 ] || return 1
    sleep 1
  done
  exec 3>&- 2>/dev/null || true
  return 0
}

port_free() {
  ss -ltn 2>/dev/null | awk '{print $4}' | grep -q ":$1\$" && return 1 || return 0
}

# 输出统一用 ANTHROPIC_API_KEY；取值优先 API_KEY，其次 AUTH_TOKEN，缺省占位
auth_var() {
  echo ANTHROPIC_API_KEY
}

auth_val() {
  if [ -n "${ANTHROPIC_API_KEY:-}" ]; then
    printf '%s' "$ANTHROPIC_API_KEY"
  elif [ -n "${ANTHROPIC_AUTH_TOKEN:-}" ]; then
    printf '%s' "$ANTHROPIC_AUTH_TOKEN"
  else
    printf '%s' "cassette-proxy"
  fi
}

save_state() {
  cat > "$STATE_FILE" <<EOF
RUN_NAME=$RUN_NAME
RECEIVER_OUT=$RECEIVER_OUT
RECEIVER_LOG=$RECEIVER_LOG
CASSETTE_LOG=$CASSETTE_LOG
CASSETTE_CONF=$CASSETTE_CONF
CASSETTE_PORT=$CASSETTE_PORT
RECEIVER_PORT=$RECEIVER_PORT
CASSETTE_STARTED_BY_US=$CASSETTE_STARTED_BY_US
RECEIVER_STARTED_BY_US=$RECEIVER_STARTED_BY_US
ANALYZE_CONF=$ANALYZE_CONF
EOF
}

load_state() {
  [ -f "$STATE_FILE" ] || return 1
  # shellcheck disable=SC1090
  . "$STATE_FILE"
  return 0
}

build_all() {
  if [ "$SKIP_BUILD" = "1" ]; then
    log "跳过编译 (SKIP_BUILD=1)"
    return 0
  fi
  local need=0
  [ "$FORCE_BUILD" = "1" ] && need=1
  [ -x "$CASSETTE_REPO/target/release/agent-cassette" ] || need=1
  [ -x "$ACTRAIL_REPO/target/release/actrail-kv-receiver" ] || need=1
  [ -x "$ACTRAIL_REPO/target/release/actrail-kv-analyze" ] || need=1
  [ "$need" = "0" ] && { log "二进制已存在，跳过编译（FORCE_BUILD=1 可强制）"; return 0; }
  log "编译 release（agent-cassette + actrail-kv）"
  (cd "$CASSETTE_REPO" && cargo build --release -p agent-cassette)
  (cd "$ACTRAIL_REPO" && cargo build --release --workspace --bins)
  ok "编译完成"
}

resolve_ports() {
  CASSETTE_PORT="${CASSETTE_PORT:-}"
  RECEIVER_PORT="${RECEIVER_PORT:-}"
  if [ -f "$CASSETTE_CONF" ]; then
    local listen receiver_url actrail_enabled
    listen="$(conf_get gateway.transport listen_addr)"
    if [ -n "$listen" ]; then
      CASSETTE_PORT="${CASSETTE_PORT:-$(port_of "$listen")}"
    fi
    actrail_enabled="$(conf_get recorder.actrail enabled)"
    receiver_url="$(conf_get recorder.actrail receiver_url)"
    if [ "${actrail_enabled,,}" = "true" ] && [ -n "$receiver_url" ]; then
      RECEIVER_PORT="${RECEIVER_PORT:-$(port_of "$receiver_url")}"
    fi
  else
    log "CASSETTE_CONF 不存在: $CASSETTE_CONF（使用默认端口）"
  fi
  CASSETTE_PORT="${CASSETTE_PORT:-18194}"
  RECEIVER_PORT="${RECEIVER_PORT:-8100}"
  log "端口: cassette=$CASSETTE_PORT receiver=$RECEIVER_PORT"
}

# 输出 agent 接入 env（在当前终端 eval）
cmd_env() {
  if ! load_state; then
    resolve_ports
  fi
  local av token model
  av="$(auth_var)"
  token="$(auth_val)"
  model="${ANTHROPIC_MODEL:-}"
  if [ -z "$model" ] && [ -f "$CASSETTE_CONF" ]; then
    model="$(conf_get proxy.provider model)"
  fi
  [ -n "$model" ] || model="default"
  cat <<EOF
# 当前终端执行: eval "\$(bash $0 --env)"
export ANTHROPIC_BASE_URL="http://127.0.0.1:$CASSETTE_PORT"
export $av="$token"
export ANTHROPIC_MODEL="$model"
EOF
}

cmd_init() {
  resolve_ports
  build_all
  mkdir -p "$KV_ROOT/run" "$(dirname "$RECEIVER_OUT")"

  CASSETTE_STARTED_BY_US=0
  RECEIVER_STARTED_BY_US=0

  if port_free "$RECEIVER_PORT"; then
    log "启动 receiver :$RECEIVER_PORT -> $RECEIVER_OUT"
    (cd "$ACTRAIL_REPO" && setsid nohup target/release/actrail-kv-receiver \
      --listen "127.0.0.1:$RECEIVER_PORT" --output "$RECEIVER_OUT" \
      >"$RECEIVER_LOG" 2>&1 < /dev/null & echo $! > "$KV_ROOT/run/.pid-anthropic-receiver")
    wait_port "$RECEIVER_PORT" || fail "receiver 未监听，见 $RECEIVER_LOG"
    RECEIVER_STARTED_BY_US=1
    ok "receiver 已启动"
  else
    if [ "$PORT_CHECK" = "true" ]; then
      fail "receiver 端口 $RECEIVER_PORT 已被占用（PORT_CHECK=true）"
    fi
    log "receiver 端口 $RECEIVER_PORT 已被占用：复用现有 receiver（默认不检查端口有效性）"
    if [ ! -f "$RECEIVER_OUT" ]; then
      log "WARN: 现有 receiver 输出文件未知，--report 前需 export RECEIVER_OUT=<ndjson 路径>"
    fi
  fi

  if port_free "$CASSETTE_PORT"; then
    log "启动 agent-cassette :$CASSETTE_PORT（配置 $CASSETTE_CONF，只读）"
    (cd "$CASSETTE_REPO" && setsid nohup target/release/agent-cassette \
      -c "$CASSETTE_CONF" start >"$CASSETTE_LOG" 2>&1 < /dev/null & echo $! > "$KV_ROOT/run/.pid-anthropic-cassette")
    wait_port "$CASSETTE_PORT" || fail "agent-cassette 未监听，见 $CASSETTE_LOG"
    CASSETTE_STARTED_BY_US=1
    ok "agent-cassette 已启动"
  else
    if [ "$PORT_CHECK" = "true" ]; then
      fail "cassette 端口 $CASSETTE_PORT 已被占用（PORT_CHECK=true）"
    fi
    log "cassette 端口 $CASSETTE_PORT 已被占用：复用现有实例（默认不检查端口有效性）"
  fi
  if curl -fsS --max-time 3 "http://127.0.0.1:$CASSETTE_PORT/healthz" >/dev/null 2>&1; then
    ok "cassette healthz ok"
  elif [ "$PORT_CHECK" = "true" ]; then
    fail "cassette healthz 失败"
  else
    log "WARN: cassette healthz 未通过（默认不视为失败，可用 --health 检查）"
  fi
  save_state
  # 记录本次用过的 conf，stop 时即使状态文件丢失也能按它清理
  echo "$CASSETTE_CONF" >> "$KV_ROOT/run/.anthropic-agent-confs.list"

  # 直接输出三个环境变量及值，供复制到启动 agent 的终端
  local av token model
  av="$(auth_var)"
  token="$(auth_val)"
  model="${ANTHROPIC_MODEL:-}"
  if [ -z "$model" ] && [ -f "$CASSETTE_CONF" ]; then
    model="$(conf_get proxy.provider model)"
  fi
  [ -n "$model" ] || model="default"

  cat <<EOF
========================================================
录制链已就绪: run=$RUN_NAME
  agent-cassette: http://127.0.0.1:$CASSETTE_PORT
  receiver ndjson: $RECEIVER_OUT

agent 指向 cassette 的环境变量（在当前终端执行下面 3 行，然后启动 agent）:
  export ANTHROPIC_BASE_URL="http://127.0.0.1:$CASSETTE_PORT"
  export $av="$token"
  export ANTHROPIC_MODEL="$model"

快捷方式（等价于上面 3 行）:
  eval "\$(bash $0 --env)"

启动 agent 执行任务；结束后执行:
  bash $0 --report
  bash $0 --stop
========================================================
EOF
}

cmd_health() {
  if ! load_state; then
    resolve_ports
  fi
  local ok=1
  local upstream_model="default"
  if [ -f "$CASSETTE_CONF" ]; then
    upstream_model="$(conf_get proxy.provider model)"
    [ -n "$upstream_model" ] || upstream_model="${ANTHROPIC_MODEL:-default}"
  else
    upstream_model="${ANTHROPIC_MODEL:-default}"
  fi

  # 1) cassette healthz
  if curl -fsS --max-time 3 "http://127.0.0.1:$CASSETTE_PORT/healthz" >/dev/null 2>&1; then
    ok "cassette healthz ok (127.0.0.1:$CASSETTE_PORT)"
  else
    log "FAIL: cassette healthz 不可达 (127.0.0.1:$CASSETTE_PORT)"
    ok=0
  fi

  # 2) receiver 端口
  if (exec 3<>"/dev/tcp/127.0.0.1/$RECEIVER_PORT") 2>/dev/null; then
    exec 3>&- 2>/dev/null || true
    ok "receiver 端口可达 (127.0.0.1:$RECEIVER_PORT)"
  else
    log "FAIL: receiver 端口不可达 (127.0.0.1:$RECEIVER_PORT)"
    ok=0
  fi

  # 3) ndjson
  if [ -n "${RECEIVER_OUT:-}" ] && [ -f "$RECEIVER_OUT" ]; then
    log "ndjson: $RECEIVER_OUT ($(wc -l < "$RECEIVER_OUT") 行)"
  else
    log "ndjson: 未找到 ${RECEIVER_OUT:-<未设置>}"
  fi

  # 4) OpenAI 兼容端点
  local code
  local openai_out="$KV_ROOT/run/health-openai-$RUN_NAME.out"
  code="$(curl -s -o "$openai_out" -w '%{http_code}' --max-time 90 \
      "http://127.0.0.1:$CASSETTE_PORT/v1/chat/completions" \
      -H 'content-type: application/json' \
      -d "{\"model\":\"$upstream_model\",\"max_tokens\":8,\"messages\":[{\"role\":\"user\",\"content\":\"ping\"}],\"stream\":false}" || true)"
  if [ "$code" = "200" ]; then
    local sse_lines
    sse_lines="$(grep -c '^data:' "$openai_out" 2>/dev/null || echo 0)"
    ok "OpenAI /v1/chat/completions HTTP 200${sse_lines:+（SSE data 行数 $sse_lines）}"
  else
    log "FAIL: OpenAI /v1/chat/completions HTTP ${code:-连接失败}"
    ok=0
  fi

  # 5) Anthropic Messages 端点
  local anthropic_out="$KV_ROOT/run/health-anthropic-$RUN_NAME.out"
  code="$(curl -s -o "$anthropic_out" -w '%{http_code}' --max-time 90 \
      "http://127.0.0.1:$CASSETTE_PORT/v1/messages" \
      -H 'content-type: application/json' \
      -H 'anthropic-version: 2023-06-01' \
      -H "x-api-key: $(auth_val)" \
      -d "{\"model\":\"$upstream_model\",\"max_tokens\":8,\"messages\":[{\"role\":\"user\",\"content\":\"ping\"}]}" || true)"
  if [ "$code" = "200" ]; then
    ok "Anthropic /v1/messages HTTP 200（协议入口正常）"
  else
    log "FAIL: Anthropic /v1/messages HTTP ${code:-连接失败}"
    ok=0
  fi

  if [ "$ok" = "1" ]; then
    ok "HEALTH: PASS"
    return 0
  fi
  log "HEALTH: FAIL"
  return 1
}

source "$SCRIPT_DIR/fullchain-deploy-linux/report.sh"

cmd_stop() {
  local confs=() outs=()
  local list="$KV_ROOT/run/.anthropic-agent-confs.list"

  # 收集 conf / receiver output 候选：状态文件 + 当前环境 + 历史 conf 列表
  if load_state; then
    [ -n "${CASSETTE_CONF:-}" ] && confs+=("$CASSETTE_CONF")
    [ -n "${RECEIVER_OUT:-}" ] && outs+=("$RECEIVER_OUT")
  fi
  [ -n "${CASSETTE_CONF:-}" ] && confs+=("$CASSETTE_CONF")
  [ -n "${RECEIVER_OUT:-}" ] && outs+=("$RECEIVER_OUT")
  if [ -f "$list" ]; then
    while IFS= read -r c; do
      [ -n "$c" ] && confs+=("$c")
    done < "$list"
  fi

  # 去重后无条件 kill：只要是用这些 conf 启动的 agent-cassette 都杀
  local seen="" seen_out="" c o killed=0
  for c in "${confs[@]:-}"; do
    case " $seen " in *" $c "*) continue ;; esac
    seen="$seen $c"
    while read -r pid; do
      kill "$pid" 2>/dev/null && killed=$((killed + 1)) || true
    done < <(pgrep -f "agent-cassette -c $c start" || true)
  done

  # receiver：状态/环境指定输出 + 所有本脚本命名的输出（requests-anthropic-agent-*）
  for o in "${outs[@]:-}"; do
    case " $seen_out " in *" $o "*) continue ;; esac
    seen_out="$seen_out $o"
    while read -r pid; do
      kill "$pid" 2>/dev/null && killed=$((killed + 1)) || true
    done < <(pgrep -f "actrail-kv-receiver .*--output $o" || true)
  done
  while read -r pid; do
    kill "$pid" 2>/dev/null && killed=$((killed + 1)) || true
  done < <(pgrep -f "actrail-kv-receiver .*--output .*requests-anthropic-agent-" || true)

  # pid 文件里记录的进程也补一刀
  for pf in "$KV_ROOT"/run/.pid-anthropic-*; do
    [ -f "$pf" ] || continue
    kill "$(cat "$pf")" 2>/dev/null || true
    rm -f "$pf"
  done

  # 先 TERM，1 秒后还有残留再 KILL
  sleep 1
  for c in $seen; do
    while read -r pid; do kill -9 "$pid" 2>/dev/null || true; done \
      < <(pgrep -f "agent-cassette -c $c start" || true)
  done
  while read -r pid; do kill -9 "$pid" 2>/dev/null || true; done \
    < <(pgrep -f "actrail-kv-receiver .*--output .*requests-anthropic-agent-" || true)
  sleep 1

  rm -f "$list" 2>/dev/null || true
  local remain=""
  for c in $seen; do
    for pid in $(pgrep -f "agent-cassette -c $c start" || true); do
      remain="$remain $pid"
    done
  done
  for pid in $(pgrep -f "actrail-kv-receiver .*requests-anthropic-agent-" || true); do
    remain="$remain $pid"
  done
  if [ -z "$remain" ]; then
    ok "stop 完成（本轮 kill=$killed，无残留）"
  else
    log "WARN: 仍有残留进程:$remain（可能由其它 conf 启动）"
  fi
}

cmd="${1:-}"
case "$cmd" in
  --init|init) cmd_init ;;
  --env|env) cmd_env ;;
  --health|health) cmd_health ;;
  --report|report)
    input="${2:-}"
    [ "$input" = "--input" ] && input="${3:-}"
    cmd_report "$input"
    ;;
  --stop|stop) cmd_stop ;;
  -h|--help|help) usage 0 ;;
  *) usage 1 ;;
esac
