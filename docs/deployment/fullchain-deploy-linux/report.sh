#!/usr/bin/env bash
# 本模块负责从采集语料生成分析与 HTML 报告，并输出当前协议的统计信息。

cmd_report() {
  local input="${1:-}"
  if [ -z "$input" ]; then
    if load_state && [ -n "${RECEIVER_OUT:-}" ] && [ -f "$RECEIVER_OUT" ]; then
      input="$RECEIVER_OUT"
    else
      input="$(ls -t "$KV_ROOT"/run/requests-anthropic-agent-*.ndjson 2>/dev/null | head -1 || true)"
    fi
  fi
  [ -n "$input" ] && [ -f "$input" ] \
    || fail "找不到 ndjson（可用 RECEIVER_OUT=... 或 report --input <file> 指定）"
  [ -n "${ANALYZE_CONF:-}" ] || ANALYZE_CONF="$ACTRAIL_REPO/examples/analyze.config.example.json"
  [ -f "$ANALYZE_CONF" ] || fail "analyze 配置不存在: $ANALYZE_CONF"
  mkdir -p "$OUT_DIR"
  log "分析 $input"
  (cd "$ACTRAIL_REPO" && \
    target/release/actrail-kv-analyze --config "$ANALYZE_CONF" \
      --input "$input" --output "$OUT_DIR/analysis.json" && \
    target/release/actrail-kv-report \
      --input "$OUT_DIR/analysis.json" --output "$OUT_DIR/report.html")
  chmod -R a+rX "$OUT_DIR"
  python3 - "$OUT_DIR/analysis.json" <<'PYEOF'
import json, sys
d = json.load(open(sys.argv[1]))
r = d["run"]
print("input", r["input_records"], "analyzed", r["analyzed_records"],
      "skipped", len(r.get("skipped_records", [])),
      "templates", len(d["templates"]), "defects", len(d["defects"]),
      "sessions", len(d["session_analysis"]["timelines"]))
for x in d.get("defects", []):
    mm = x.get("mismatch", {})
    print(" defect", x.get("id", "")[:16], mm.get("pattern"),
          "affected", x.get("affected_count"), "blocked", x.get("blocked_stable_bytes"))
PYEOF
  cat <<EOF
========================================================
报告完成:
  analysis: $OUT_DIR/analysis.json
  report:   $OUT_DIR/report.html
  浏览器:   http://<IP>:7777/$(basename "$OUT_DIR")/report.html
========================================================
EOF
}
