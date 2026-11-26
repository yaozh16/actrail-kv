#!/usr/bin/env bash
# 同一个已编译宿主替换 C ABI 实现后重启，验证版本和缺失符号拒绝。
set -euo pipefail
repo_dir=$(cd "$(dirname "$0")/../.." && pwd)
cd "$repo_dir"
test_dir=$(mktemp -d)
trap 'rm -rf "$test_dir"' EXIT
cargo build --release -p actrail-kv-sdk --example sdk_probe
host="${CARGO_TARGET_DIR:-target}/release/examples/sdk_probe"
host_hash=$(sha256sum "$host")
source_file=tests/fixtures/sdk/abi_fixture.c
include_dir=crates/sdk/native/contracts/include
cc -shared -fPIC -Wall -Wextra -Werror -I "$include_dir" "$source_file" -o "$test_dir/analyzer.so" -DACTRAIL_TEST_REVISION=1
[[ $("$host" "$test_dir/analyzer.so") == '{"revision":1}' ]]
cc -shared -fPIC -Wall -Wextra -Werror -I "$include_dir" "$source_file" -o "$test_dir/next.so" -DACTRAIL_TEST_REVISION=2
mv "$test_dir/next.so" "$test_dir/analyzer.so"
[[ $("$host" "$test_dir/analyzer.so") == '{"revision":2}' ]]
[[ "$host_hash" == "$(sha256sum "$host")" ]]
cc -shared -fPIC -Wall -Wextra -Werror -I "$include_dir" "$source_file" -o "$test_dir/incompatible.so" -DACTRAIL_TEST_ABI=2
if "$host" "$test_dir/incompatible.so" >"$test_dir/output" 2>"$test_dir/error"; then
    echo 'incompatible ABI unexpectedly loaded' >&2
    exit 1
fi
rg -q 'ABI|Abi' "$test_dir/error"
cc -shared -fPIC -Wall -Wextra -Werror -I "$include_dir" "$source_file" -o "$test_dir/missing.so" -DACTRAIL_TEST_MISSING
if "$host" "$test_dir/missing.so" >"$test_dir/output" 2>"$test_dir/error"; then
    echo 'library with missing operation unexpectedly loaded' >&2
    exit 1
fi
rg -q 'actrail_kv_analyzer_analyze' "$test_dir/error"
echo 'PASS: unchanged host, compatible replacement, ABI and symbol rejection'
