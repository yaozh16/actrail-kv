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
    -H 'x-actrail-endpoint-key: e2e-primary' \
    -H 'x-actrail-agent-key: e2e-agent' \
    --data-binary "$payload" \
    "http://$listen_address/requests")
  [[ "$status" == "202" ]]
done

session_id='<img src=x onerror=alert(1)>'
session_stable='You are an append-only agent. Keep this established history byte-for-byte across later requests.'
for messages in \
  "$(jq -cn --arg stable "$session_stable" '[{role:"system",content:$stable}]')" \
  "$(jq -cn --arg stable "$session_stable" '[{role:"system",content:$stable},{role:"user",content:"first appended turn"}]')" \
  "$(jq -cn --arg stable "$session_stable" '[{role:"system",content:($stable + " rewritten")},{role:"user",content:"first appended turn"}]')"; do
  payload=$(jq -cn --argjson messages "$messages" '{model:"e2e-session-model",messages:$messages}')
  status=$(curl --silent --output /dev/null --write-out '%{http_code}' \
    -H 'content-type: application/json' \
    -H 'x-actrail-source: e2e-session-hook' \
    -H 'x-actrail-endpoint-key: e2e-session' \
    -H "x-actrail-session-id: $session_id" \
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

# 裸文件名输出支持创建和原子替换，且不能覆盖分析输入。
report_bin="$repo_dir/target/debug/actrail-kv-report"
(
  cd "$test_dir"
  "$report_bin" --input analysis.json --output relative-report.html
  cmp report.html relative-report.html
  "$report_bin" --input analysis.json --output relative-report.html
  cmp report.html relative-report.html
  if "$report_bin" --input analysis.json --output analysis.json 2>/dev/null; then
    echo "report overwrote its input" >&2
    exit 1
  fi
  cmp analysis.json analysis-repeat.json
)

# 旧文本阈值继续读写和校验，但不改变实际诊断结果。
jq '.text_similarity_threshold = 0.0 | .top_k = 3' \
  examples/analyze.config.example.json > "$test_dir/legacy.config.json"
target/debug/actrail-kv-analyze \
  --config "$test_dir/legacy.config.json" \
  --input "$test_dir/requests.ndjson" \
  --output "$test_dir/analysis-legacy.json"
jq -e '.run.options.text_similarity_threshold == 0.0' "$test_dir/analysis-legacy.json" >/dev/null
jq '.run.options.text_similarity_threshold = 0.8' "$test_dir/analysis-legacy.json" > "$test_dir/legacy-normalized.json"
jq '.' "$test_dir/analysis.json" > "$test_dir/baseline-normalized.json"
cmp "$test_dir/baseline-normalized.json" "$test_dir/legacy-normalized.json"
target/debug/actrail-kv-report \
  --input "$test_dir/analysis-legacy.json" \
  --output "$test_dir/legacy-report.html"

[[ $(wc -l < "$test_dir/requests.ndjson") -eq 7 ]]
[[ $(jq -s --arg id "$session_id" '[.[] | select(.session_id == $id)] | length' "$test_dir/requests.ndjson") -eq 3 ]]
[[ $(jq '.defects | length' "$test_dir/analysis.json") -ge 1 ]]
top_defect_id=$(jq -r '.top_k[0] // empty' "$test_dir/analysis.json")
[[ -n "$top_defect_id" ]]
jq -e --arg id "$top_defect_id" 'any(.defects[]; .id == $id)' "$test_dir/analysis.json" >/dev/null
jq -e 'all(.defects[]; (.mismatch.variants | length) >= 2 and .recovered_stable.support_count == .comparable_count)' "$test_dir/analysis.json" >/dev/null
jq -e 'any(.templates[]; .comparison_group.endpoint_key == "e2e-primary" and .comparison_group.agent_key == "e2e-agent")' "$test_dir/analysis.json" >/dev/null
jq -e '.conditional_local_sites | type == "array"' "$test_dir/analysis.json" >/dev/null
jq -e '.session_analysis.session_record_count == 3 and (.session_analysis.timelines | length) == 1' "$test_dir/analysis.json" >/dev/null
jq -e 'any(.session_analysis.timelines[0].transitions[]; .outcome.kind == "normal_append")' "$test_dir/analysis.json" >/dev/null
jq -e 'any(.session_analysis.timelines[0].transitions[]; .outcome.kind == "history_changed")' "$test_dir/analysis.json" >/dev/null
jq -e '(.session_analysis.history_sites | length) >= 1' "$test_dir/analysis.json" >/dev/null
grep -q 'LLM KV 缓存结构诊断' "$test_dir/report.html"
grep -q '<strong>P1</strong>' "$test_dir/report.html"
grep -q '<strong>X</strong>' "$test_dir/report.html"
grep -q '<strong>P2</strong>' "$test_dir/report.html"
grep -q '&lt;script&gt;alert(1)&lt;/script&gt;' "$test_dir/report.html"
grep -q '条件性局部位点' "$test_dir/report.html"
grep -q 'Session 时间线' "$test_dir/report.html"
grep -q '&lt;img src=x onerror=alert(1)&gt;' "$test_dir/report.html"
if grep -q '<script>' "$test_dir/report.html"; then
  echo "unsafe script content found in report" >&2
  exit 1
fi
