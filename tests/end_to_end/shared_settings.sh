#!/usr/bin/env bash
# 本脚本验证四个命令使用同一份部分或缺失的配置完成采集至报告流程。
set -euo pipefail

repo_dir=$(cd "$(dirname "$0")/../.." && pwd)
test_dir=$(mktemp -d)
receiver_pid=""

cleanup() {
  if [[ -n "$receiver_pid" ]]; then
    kill -TERM "$receiver_pid" 2>/dev/null || true
    wait "$receiver_pid" 2>/dev/null || true
  fi
  rm -rf "$test_dir"
}
trap cleanup EXIT

cd "$repo_dir"
cargo build -q --workspace --bins >"$test_dir/build.log" 2>&1 || { tail -n 30 "$test_dir/build.log" >&2; exit 1; }

settings="$test_dir/project/settings.toml"
mkdir -p "$(dirname "$settings")"
cat > "$settings" <<'TOML'
[receiver]
future_setting = true
[future_service]
enabled = true
TOML
target/debug/actrail-kv-receiver --config "$settings" --listen 127.0.0.1:0 \
  2>"$test_dir/receiver.log" &
receiver_pid=$!
for _ in $(seq 1 100); do
  if grep -q '^listening on ' "$test_dir/receiver.log"; then break; fi
  sleep 0.05
done
listen_address=$(sed -n 's/^listening on //p' "$test_dir/receiver.log" | head -n 1)
test -n "$listen_address"
grep -Fq 'warning: unknown configuration field `receiver.future_setting`' "$test_dir/receiver.log"
! grep -Fq 'future_service' "$test_dir/receiver.log"

python3 - "$settings" <<'PYTOML'
import sys, tomllib
with open(sys.argv[1], 'rb') as stream:
    settings = tomllib.load(stream)
assert settings['storage']['backend'] == 'sqlite'
assert settings['receiver']['listen'] == '127.0.0.1:8080'
assert 'analyzer' not in settings
PYTOML

test -f "$test_dir/project/captures.sqlite"

for session in selected other; do
  response=$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    -H 'Content-Type: application/json' \
    -H 'X-Actrail-Endpoint-Key: smoke' \
    -H "X-Actrail-Session-Key: $session" \
    --data '{"model":"test","messages":[{"role":"user","content":"Hello"}]}' \
    "http://$listen_address/requests")
  test "$response" = 202
done

cat >> "$settings" <<'TOML'

[analyzer]
output = "selected.json"
[analyzer.options.corpus]
max_records = 9
future_limit = 123
[analyzer.selection]
session_keys = ["selected"]
[[analyzer.selection.time_ranges]]
from = "2000-01-01"
future_bound = true
[report]
locale = "en-US"
TOML

target/debug/actrail-kv-analyze --config "$settings" 2>"$test_dir/analyze.log"
grep -Fq 'warning: unknown configuration field `analyzer.options.corpus.future_limit`' "$test_dir/analyze.log"
grep -Fq 'warning: unknown configuration field `analyzer.selection.time_ranges[0].future_bound`' "$test_dir/analyze.log"
target/debug/actrail-kv-export --config "$settings"
target/debug/actrail-kv-report --config "$settings"

jq -e '.run.input_records == 1 and .run.options.corpus.max_records == 9 and .run.schema_version == "0.1.1"' "$test_dir/project/selected.json" >/dev/null
test "$(wc -l < "$test_dir/project/exported.ndjson")" -eq 2
test -s "$test_dir/project/report.html"
test -s "$test_dir/project/report.md"
python3 - "$settings" <<'PYTOML'
import sys, tomllib
with open(sys.argv[1], 'rb') as stream:
    settings = tomllib.load(stream)
assert settings['analyzer']['output'] == 'selected.json'
assert settings['analyzer']['options']['corpus']['max_records'] == 9
assert settings['analyzer']['options']['corpus']['future_limit'] == 123
assert settings['receiver']['future_setting'] is True
assert settings['future_service']['enabled'] is True
assert settings['analyzer']['options']['corpus']['max_record_bytes'] == 8388608
assert settings['analyzer']['options']['stability']['min_stable_support'] == 3
assert settings['analyzer']['selection']['session_keys'] == ['selected']
assert settings['analyzer']['selection']['time_ranges'][0]['future_bound'] is True
assert settings['report']['locale'] == 'en-US'
assert settings['export']['output'] == 'exported.ndjson'
PYTOML
