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
cargo build --workspace --bins

settings="$test_dir/project/settings.json"
target/debug/actrail-kv-receiver --config "$settings" --listen 127.0.0.1:0 \
  2>"$test_dir/receiver.log" &
receiver_pid=$!
for _ in $(seq 1 100); do
  if grep -q '^listening on ' "$test_dir/receiver.log"; then break; fi
  sleep 0.05
done
listen_address=$(sed -n 's/^listening on //p' "$test_dir/receiver.log" | head -n 1)
test -n "$listen_address"

jq -e '.storage.backend == "sqlite" and .receiver.listen == "127.0.0.1:8080" and (has("analyzer") | not)' "$settings" >/dev/null
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

jq '.analyzer = {output: "selected.json", selection: {session_keys: ["selected"]}} | .report = {locale: "en-US"}' \
  "$settings" > "$test_dir/partial.json"
mv "$test_dir/partial.json" "$settings"

target/debug/actrail-kv-analyze --config "$settings"
target/debug/actrail-kv-export --config "$settings"
target/debug/actrail-kv-report --config "$settings"

jq -e '.run.input_records == 1' "$test_dir/project/selected.json" >/dev/null
test "$(wc -l < "$test_dir/project/exported.ndjson")" -eq 2
test -s "$test_dir/project/report.html"
test -s "$test_dir/project/report.md"
jq -e '.analyzer.output == "selected.json" and .analyzer.selection.session_keys == ["selected"] and .report.locale == "en-US" and .export.output == "exported.ndjson"' "$settings" >/dev/null
