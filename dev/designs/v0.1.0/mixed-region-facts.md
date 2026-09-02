<!-- 本文件定义混合区域内容事实的判定边界。 -->
# 混合区域内容事实设计

## 判定规则

`content::analyze` 的内容证据只取 parts 全部为 `binding` 的变体：

```text
pure_binding_variants = variants where
  parts 非空 且 每个 part.kind == "binding"
```

- `pure_binding_variants` 不足两个或 shape 不一致：不输出内容事实。
- 达到两个及以上且内容指纹不同：输出 `content_variation` / `fixed_variants` 事实与对应启示。
- 缺失、unmatched 变体仍参与 `sequence::analyze`，贡献 `insertion_deletion` / `reorder` 证据。
- 两者同时成立时 pattern 为 `mixed`；只有序列证据时保持 `insertion_deletion` / `reorder`。

## 影响

- 不改变纯内容变体、纯插入缺失、纯重排的既有判定。
- 提高混合成员场景的事实完整性，不新增问题数量。
