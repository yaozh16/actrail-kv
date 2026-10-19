<!-- 本文件记录同一 session 内上下文刷新与前缀复用离群检测需求。 -->
# 会话上下文刷新检测需求

## 背景

同一 `session_key` 内的连续 LLM 请求通常应保持 append-only：

```text
A → AB → ABC → ABCD
```

当请求变为：

```text
A → AB → ABC → ABCD → EF → EFG → EFGH
```

`ABCD → EF` 表示上一轮上下文没有被下一轮请求保留。该变化可能来自上下文压缩、
subagent 分叉、任务切换或运行时重置。它不一定错误，但会废弃原有 KVC 前缀，
因此必须在会话报告中作为 warning 显式呈现，由用户判断是否符合预期。

## 目标

1. 以 `session_key` 为唯一会话边界，按采集时间分析请求连续性。
2. 从会话第二个请求开始，为每个请求计算与上一条请求的前缀复用度。
3. 将前缀复用度低于完整保留的请求标记为上下文刷新离群点。
4. 在 `analysis.json` 中输出逐请求证据和会话级刷新次数。
5. 在 `report.html` 中展示发生刷新的会话、具体轮次和废弃前缀规模。
6. 上下文压缩与 subagent 创建只要改变同一 session 的上下文连续性，也产生 warning。

## 指标口径

对同一 session 的相邻请求 `previous`、`current`：

```text
previous_total_bytes =
  上一条请求全部内容性上下文单元字节
  （VisibleText / ToolDefinition / ToolCall / ToolResult）

retained_prefix_bytes =
  previous 与 current 内容性单元连续公共前缀的字节

prefix_reuse_ratio =
  retained_prefix_bytes / previous_total_bytes

abandoned_prefix_bytes =
  previous_total_bytes - retained_prefix_bytes
```

`prefix_reuse_ratio` 衡量“上一条上下文被保留了多少”，不是公共前缀在当前请求中的占比。
正常 append-only 时，即使本轮新增大量内容，该值仍为 `1.0`。

当 `previous_total_bytes > 0` 且 `retained_prefix_bytes < previous_total_bytes` 时，当前请求是
前缀复用离群点，并产生一次上下文刷新 warning。判定不使用可调阈值、中位数或标准差。

示例：

| 当前请求 | 与上一条的公共前缀 | 前缀复用度 | 结果 |
|---|---|---:|---|
| `A` | 无前序 | 无 | 跳过 |
| `AB` | `A` | 1.0 | 正常追加 |
| `ABC` | `AB` | 1.0 | 正常追加 |
| `ABCD` | `ABC` | 1.0 | 正常追加 |
| `EF` | 0 | 0.0 | 刷新 warning |
| `EFG` | `EF` | 1.0 | 正常追加 |
| `EFGH` | `EFG` | 1.0 | 正常追加 |

该 session 的刷新次数为 `1`，刷新位置为 `ABCD → EF`。

## 会话与事件

- 同一 `session_key` 按 `captured_at` 升序排列；时间相同时按输入行序稳定排序。
- 会话首请求没有前序，不计算前缀复用度，也不产生刷新 warning。
- 只有一条有效请求的 session 不产生刷新事件。
- 每个离群相邻请求对产生一个事件；事件通过现有
  `append / fork / reorder / reset` 类型说明上下文如何变化。
- `context_refresh_count` 等于该 session 中离群相邻请求对的数量。
- 同时输出公共前缀、上一条上下文总字节、废弃前缀字节和事件类型。

## 报告要求

报告中的会话区块需要展示：

1. 有上下文刷新的 session 数、刷新 warning 总数和被废弃前缀总字节；
2. session 的刷新次数、最低前缀复用度和最早发生刷新的轮次；
3. 逐事件展示前后请求、事件类型、公共前缀、前缀复用度与废弃字节；
4. 刷新 warning 使用独立风险样式，不归入上下文结构 `defect`，也不自动标记为 error；
5. 全部为 append-only 的 session 明确显示“未发现上下文刷新”；
6. 报告文案说明 compact、subagent 等机制也可能触发 warning，由用户判断是否接受。

## 边界

- 当前需求不判断是否接近模型上下文压缩上限。没有可靠上下文窗口或压缩阈值时，
  不计算、不输出“压缩压力未知”等占位状态。
- 不根据文本关键词把刷新归因为 compact、subagent 或任务切换；只报告可观察的结构变化。
- 不同 `session_key` 之间不比较，也无法仅凭现有输入关联父 session 与独立 subagent session。
- 无 `session_key` 的请求不进入本检测。
- UTF-8 bytes 是可观察结构代理，不代表 token、真实 KV miss、延迟或金额收益。

## 验收

- 示例 `A / AB / ABC / ABCD / EF / EFG / EFGH` 只产生一次上下文刷新，
  刷新位置为 `ABCD → EF`，前缀复用度序列为 `1 / 1 / 1 / 0 / 1 / 1`。
- 全部 append-only 的 session 不产生刷新 warning。
- 同一 session 的请求跨 endpoint 或 model 时仍按一个 session 分析。
- 输入 NDJSON 存在被投影跳过的记录时，session 不得与错误的请求序列配对。
- 同一 session 中上下文压缩或 subagent 导致的 fork/reset 产生 warning，但不标记为 error。
- 旧版 `analysis.json` 缺少新增字段时仍可解析；新版字段缺失时报告使用已有事件类型回退。
- report 对所有来自 `analysis.json` 的文本继续执行 HTML escape。
