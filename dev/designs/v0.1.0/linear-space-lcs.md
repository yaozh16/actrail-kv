<!-- 本文件定义 Hirschberg 实现。 -->
# 文本 LCS 线性空间设计

## 算法

在 `extractor.rs` 用 Hirschberg 线性空间 LCS 替换全矩阵回溯：

- 递归按左序列二分；
- 每次用两行滚动 DP 计算正向/反向 LCS 长度行；
- 选择分割点后递归左右两段，返回 `(left_idx, right_idx)` 单调配对；
- 配对结果写回 `matched` 数组。

## 边界

- `left == right` 仍走线性 fast path；
- cell 预算在进入递归前按 `(len+1)^2` 检查与扣除，语义不变；
- 输出为一条合法的 LCS 配对，不承诺与旧矩阵回溯的 tie-break 逐位一致。
