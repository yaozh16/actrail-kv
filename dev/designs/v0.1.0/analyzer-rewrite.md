<!-- 本文件定义 analyzer 统一缺陷模型重写的模块、数据流和公共产物设计。 -->
# Analyzer 统一缺陷模型重写设计

## 输入 envelope

```text
CapturedRequest
├─ captured_at
├─ source?
├─ comparison
│  ├─ endpoint_key
│  ├─ agent_key?
│  ├─ model_deployment_key?
│  └─ kv_namespace?
└─ payload
```

缺失 `endpoint_key` 的记录由 analyzer 明确跳过并写入原因。receiver 不解释 comparison 字段。

## 核心数据流

```text
CapturedRequest
  → ComparisonGroup
  → CacheSequence
  → SymbolicTemplate + MemberCoordinateMap
  → SymbolicMemberSequence
  → MismatchEpisode(P1, X variants, P2)
  → ContextDefect
  → stable Top K
```

## 模板坐标

`SymbolicTemplate` 保存稳定坐标、槽位坐标和层级 source locator。`MemberCoordinateMap` 对每个模板成员记录该坐标在成员中的 unit/text range 或 gap。

`SymbolicMemberSequence` 的 atom 为：

- `StableRef`：引用模板稳定坐标；
- `SlotBinding`：同一槽位的成员值；
- `UnmatchedRun`：无法绑定的连续成员区域；
- `Gap`：该成员在模板坐标处缺失。

atom 保留完整 digest、UTF-8 bytes、source range、层级 scope 和可选稳定 identity。

## 失配 episode

locator 先形成唯一 region，再运行事实分析：

```text
MismatchEpisode
├─ left_boundary
├─ variants[]
├─ recovery_anchor
├─ recoverable_stable
└─ facts[]
```

- `variants[]` 直接跨模板成员聚合，每个成员贡献一个 canonical X variant，空区域也是显式变体。
- `facts[]` 可以同时包含内容变化、少数固定版本、结构化数据等价、插入/缺失、移动/重排和 scope。
- 同一区域的多个事实不得拆成多个 ContextDefect。
- 重排事实必须有稳定、唯一的一一对应证据。

## 公共产物

```text
AnalysisResult
├─ run
├─ templates[]
├─ defects[]
└─ top_k[]              # ordered defect IDs

ContextDefect
├─ id
├─ comparison_group
├─ template_id
├─ mismatch
│  ├─ variants[]
│  └─ facts[]
├─ recovered_stable
├─ actual_prefix_bytes
├─ potential_prefix_bytes
├─ affected_count
├─ confidence
├─ score
└─ insights[]
```

公共 schema 直接替换 v0.1.0 未发布产物；删除 `FindingCause`、单一 source、平行 excerpts、重复 TopK score 和 cause-based recommendation。

## 聚合、身份与排序

- `affected_count = P2 可比较成员数 - 最大 X 变体组成员数`。
- `blocked_stable_bytes` 取受影响成员可恢复连续 bytes 的保守值。
- `score = blocked_stable_bytes × affected_count × confidence`。
- identity 使用 comparison group、稳定模板 family、逻辑 X 边界、完整 P2 anchor chain 和局部坐标形状。
- identity 排除 raw X、成员/medoid ID、绝对 unit index、excerpt、fact 名称和 insight 文案。
- 排序依次使用 score、affected、blocked 降序和稳定 ID 升序；`top_k` 是全排序的前缀。

## 模块职责

```text
analyzer/src/
├─ model/
│  ├─ corpus/             envelope 加载与 comparison group
│  └─ projection/         模型可见层级投影
├─ discovery/
│  ├─ candidate/          有界模板候选召回
│  └─ template/           模板坐标、成员映射、符号序列
├─ diagnosis/
│  ├─ episode/            P1/X/P2 定位与跨成员聚合
│  ├─ facts/              content、sequence、representation 事实
│  └─ identity/           稳定 region identity
├─ ranking/               score、稳定排序、Top K
└─ run/                   预算、流水线与 artifact 映射
```

目录保持层级职责单一，每个目录不超过 8 个文件；超过 500 行的文件必须继续拆分。

## 实施顺序

1. 重建 artifacts input/output DTO。
2. 重建 comparison group、模板坐标和成员符号序列。
3. 实现唯一 P1/X/P2 locator 与跨成员变体聚合。
4. 实现 content、sequence、representation facts 和中文 insights。
5. 重建 identity、ranking、reporter 和 CLI 映射。
6. 按 requirements 测试矩阵补齐单元、性质、规模和 E2E 测试。
7. 全部通过后更新 current-state docs。
