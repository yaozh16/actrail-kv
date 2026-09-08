<!-- 本文件记录会话内 prefix-switch（AAAABBCC）诊断需求。 -->
# 会话内 Prefix-Switch 诊断需求

## 状态

- 本方案已失效，由 v0.1.1 的 `session_id + session_analysis` 线性时间线设计取代。
- 当前要求见 `dev/requirements/v0.1.1/target.md` 和 `dev/requirements/v0.1.1/feature-integration.md`。

## 背景

外部采集可携带 session id（新增可选 `X-Actrail-Session-Key`）。同一会话的连续请求本应保持 append-only，使前缀缓存连续命中；当上下文被重写、压缩、fork 或元素重排时，会出现“长稳定前缀后中段切换、尾段再次稳定”（AAAABBCC）的结构，造成可量化的前缀复用损失。

本需求只做**证据分类与量化**，不判断业务上是否合理；用户换任务导致的切换与生成顺序不稳定导致的切换一律只输出事件类型与字节证据。

## 目标

1. receiver 接收可选 `X-Actrail-Session-Key`，envelope 落盘 `session_key`（不进 comparison group）。
2. analyzer 按 session 对连续请求做相邻对分析，输出：
   - 事件类型：`append` / `fork` / `reorder` / `reset`；
   - 最长公共前缀（units/bytes）；
   - 切换后需重算字节与可恢复稳定字节（P2/CC 证据）。
3. `reorder` 仅在单位多重集可唯一对应时声明；有重复单位时不作 reorder 归因。
4. 输出按 session 聚合的 `session_reports`，按“切换后稳定字节浪费”降序。
5. report 渲染证据表，不做业务建议。

## 验收

- 含 `session_key` 的 4 条连续请求（append、fork、reorder、reset 各一）能被正确分类并给出非零字节证据。
- 无 `session_key` 的记录不参与会话诊断，也不影响模板分析。
- `reorder` 判定不依赖业务字段名或值格式；重复单位时不声称 reorder。
- 旧版 analysis.json（无 session_reports 字段）仍可解析。
