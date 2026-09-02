<!-- 本文件记录 v0.1.0 将编译期写死配置提升为 JSON 配置文件的需求。 -->
# 配置文件化需求

## 背景

- analyze 的算法阈值与资源预算当前写死在 `AnalysisOptions::default()`，CLI 只暴露 `--top-k` 与 `--comparison-window-seconds`。
- 真实长文本语料（约 14.6KB system prompt）两两对齐需要约 2.15 亿 cells，超过写死的单次 200 万 cells 预算，对应 cohort 被跳过，无法评估该语料是否存在结构缺陷。
- receiver 的请求体上限 `8 MiB` 与监听地址写死；监听地址虽然可由 CLI 覆盖，但没有统一配置入口。
- report 无可调算法参数，不进入本需求范围。

## 目标

1. analyze 的全部 17 项参数可通过 JSON 配置文件覆盖，CLI 显式参数仍可继续覆盖文件。
2. receiver 的监听地址与请求体上限可通过 JSON 配置文件覆盖，`--output` 保持 CLI 必填。
3. 提供 `Config` 结构体与 `to_json` / `from_json`（含格式校验），字段未知时拒绝加载。
4. 不提供配置文件时，行为与旧版本一致（默认值保持同一来源）。
5. `analysis.json` 的 `run.options` 快照继续记录本次实际生效参数，保证可追溯。

## 验收标准

- `actrail-kv-analyze --config <file>` 在放开对齐预算后可分析 14 条真实语料，不再出现因单次对齐 cells 超限导致的整 cohort 跳过（其它可解释跳过仍允许），并生成新报告文件。
- 新报告落盘到新目录，不覆盖 `run-20260902` 旧报告。
- 无 `--config` 参数时 analyze/receiver 输出与旧版一致。
- JSON 配置包含未知字段时报错退出，包含非法数值（如 0 预算、越界比例）时报错退出。
