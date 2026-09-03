<!-- 本文件定义多 region 迭代的设计。 -->
# 同一模板多 Region 设计

## 流程

`diagnose_template` 改为循环：

```text
cursor = 0
while cursor < coordinates.len():
  (x_start, search) = first_mismatch_from(cursor)
  recovery = find_recovery(from search.max(cursor))
  candidate = build_candidate(x_start, recovery)
  cursor = max(recovery.end, x_start+1)
```

原单 region 的定位/变体聚合逻辑收敛到 `build_candidate`，返回单个 `DefectCandidate`。

## 边界

- 每个 region 只生成一个问题；同一 region 的内容/序列/等价事实合并；
- 后置 region 的 P1 包含其成员自身的早期槽位值，语义上表示“前一问题修复后可继续延伸的前缀”，只做证据不做因果承诺。
