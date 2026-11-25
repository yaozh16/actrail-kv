# 本文件读取 agent-cassette 联调脚本所需的配置字段。

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
