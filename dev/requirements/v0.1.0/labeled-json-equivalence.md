<!-- 本文件记录带标签 JSON 等价表示需求。 -->
# 带标签 JSON 等价表示需求

## 背景

`structured_data_equivalent` 目前要求整个槽位文本可解析为 JSON；真实 prompt 常见 `Config JSON: {...}` 形态，标签并入槽位后无法触发等价证据。

## 目标

在整段无法解析时，允许剥离**完全一致的前导标签**后按剩余 JSON 判定严格等价；标签不一致不得判等价。

## 验收

- `Config JSON: {"a":1,"b":2}` 与键序/空白变体可触发 `structured_data_equivalent`；
- JSON 数组重排、值变化、标签不同的 case 不触发等价事实。
