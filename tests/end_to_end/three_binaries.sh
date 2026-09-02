#!/usr/bin/env bash
# 本脚本真实串联三个二进制，验收 HTTP 接收、离线分析、静态报告及输出安全。
set -euo pipefail

repo_dir=$(cd "$(dirname "$0")/../.." && pwd)
test_dir=$(mktemp -d)
receiver_pid=""

cleanup() {
  if [[ -n "$receiver_pid" ]] && kill -0 "$receiver_pid" 2>/dev/null; then
    kill -TERM "$receiver_pid" 2>/dev/null || true
    wait "$receiver_pid" 2>/dev/null || true
  fi
  rm -rf "$test_dir"
}
trap cleanup EXIT

cd "$repo_dir"
cargo build --workspace --bins

target/debug/actrail-kv-receiver \
  --listen 127.0.0.1:0 \
  --output "$test_dir/requests.ndjson" \
  2>"$test_dir/receiver.log" &
receiver_pid=$!

for _ in $(seq 1 100); do
  if grep -q '^listening on ' "$test_dir/receiver.log"; then
    break
  fi
  sleep 0.05
done

listen_address=$(sed -n 's/^listening on //p' "$test_dir/receiver.log" | head -n 1)
if [[ -z "$listen_address" ]]; then
  echo "receiver did not start" >&2
  exit 1
fi

stable_text='<script>alert(1)</script> This stable instruction block is deliberately longer than sixty-four UTF-8 bytes. Keep every rule and example in exactly the same order for reuse.'
for volatile in 'northern account' '南方客户' 'ocean workspace' 'Δοκιμή tenant'; do
  payload=$(jq -cn \
    --arg volatile "$volatile" \
    --arg stable "$stable_text" \
    '{model:"e2e-model",messages:[{role:"system",content:$volatile},{role:"system",content:$stable}]}')
  status=$(curl --silent --output /dev/null --write-out '%{http_code}' \
    -H 'content-type: application/json' \
    -H 'x-actrail-source: e2e-instrumentation' \
    --data-binary "$payload" \
    "http://$listen_address/requests")
  [[ "$status" == "202" ]]
done

kill -TERM "$receiver_pid"
wait "$receiver_pid"
receiver_pid=""

target/debug/actrail-kv-analyze \
  --input "$test_dir/requests.ndjson" \
  --output "$test_dir/analysis.json" \
  --top-k 3
target/debug/actrail-kv-analyze \
  --input "$test_dir/requests.ndjson" \
  --output "$test_dir/analysis-repeat.json" \
  --top-k 3
cmp "$test_dir/analysis.json" "$test_dir/analysis-repeat.json"
target/debug/actrail-kv-report \
  --input "$test_dir/analysis.json" \
  --output "$test_dir/report.html"

[[ $(wc -l < "$test_dir/requests.ndjson") -eq 4 ]]
[[ $(jq '.findings | length' "$test_dir/analysis.json") -ge 1 ]]
[[ $(jq -r '.top_k[0].finding_id // empty' "$test_dir/analysis.json") != "" ]]
grep -q 'LLM KV 缓存结构诊断' "$test_dir/report.html"
grep -q '&lt;script&gt;alert(1)&lt;/script&gt;' "$test_dir/report.html"
if grep -q '<script>' "$test_dir/report.html"; then
  echo "unsafe script content found in report" >&2
  exit 1
fi
