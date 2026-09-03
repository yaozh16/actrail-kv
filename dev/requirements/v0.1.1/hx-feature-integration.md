<!-- 本文件记录 feat/hx-development 最新能力并入 v0.1.1 的范围与验收要求。 -->
# feat/hx-development 特性集成要求

## 状态

- 已批准开发。
- 集成结果保持 v0.1.1 的 `session_id / session_analysis / conditional_local_sites` 公共协议。
- 源分支中与当前协议重叠的旧 Session 和多 region 实现不形成平行模型。

## 目标

在不改变 v0.1.1 Session 前缀延续和 direct/conditional 多位点语义的前提下，纳入以下独立能力：

1. 固定内容版本数量阈值可以通过 Analyze 配置调整，并写入运行快照。
2. 识别相同非 JSON 标签之后、JSON AST 等价但文本表示不同的模型可见内容。
3. 请求中部插入或删除后，候选相似度采样仍能稳定召回同模板请求。
4. 对线性空间文本 LCS 设置兼容性门禁；只有保持当前确定性 tie-break 和既有模板身份时才允许替换。
5. 引入真实 Agent 请求 case fixture 作为回归语料。

## 保留的 v0.1.1 语义

- Session Header 只使用 `X-Actrail-Session-Id`，落盘字段只使用 `session_id`。
- Session 结果只使用 `session_analysis`；不得新增 `session_key`、`session_reports` 或平行事件协议。
- 每个模板只有第一个 episode 是 direct defect；后续 episode 是 conditional local site，不计 score、不进入 Top K。
- Session transition 的失效量继续以 `previous_observable_bytes - preserved_prefix_bytes` 表达，不引入 prefix-cut 或 block-run 替代指标。

## 功能要求

### 固定版本阈值

- Analyze 配置新增正整数 `fixed_variant_max`，默认值为 `3`。
- 默认值保持现有 `FixedVariants` 判定行为。
- 值为零必须拒绝；配置、CLI 合并后的有效值必须写入 `run.options`。
- 该阈值只影响 content fact 分类，不改变模板发现、episode 身份和 Session 分析。

### 标签后 JSON 等价

- 只有所有可比较文本的标签字节完全一致、标签非空、标签后的完整剩余文本均可解析为 JSON，且 JSON AST 严格相等时，才产生结构化等价事实。
- 标签不同、JSON 后存在额外后缀、数组顺序改变或数值改变时不得声称等价。
- 整段文本本身是 JSON 的既有行为保持不变。

### 抗位移候选采样

- 文本 fingerprint 使用确定性的 bottom-k 内容采样，不依赖等距位置。
- bottom-k 必须真实保留最小的 k 个去重 hash；不得因堆方向错误保留非目标集合。
- 局部中部插入或删除不能使其余大量相同内容整体错位并失去候选召回。
- 输入顺序、hash 遍历顺序和重复 shingle 不改变输出 fingerprint。

### LCS 兼容性门禁

- 重复字符存在多条最优 LCS 时，输出映射必须与 v0.1.1 合并前的 canonical tie-break 一致。
- alignment cell admission、累计预算、UTF-8 source range 和确定性输出契约保持不变。
- 远端 Hirschberg split rule 无法保持既有重复字符 tie-break，因此本次集成保留当前矩阵实现，不以输出漂移换取内存优化。

### 回归语料

- case fixture 必须是测试所需的最小完整请求，不包含与断言无关的敏感或环境特定内容。
- workspace、JSON、插入、自然文本、策略和 Session 场景必须通过真实文件入口运行。
- Session fixture 必须使用 `session_id`，断言当前 `session_analysis`，不得保留旧协议字段。

## 已失效决策索引

- `session_key / session_reports` 由 v0.1.1 `session_id / session_analysis` 取代。
- 所有 P1/X/P2 region 都作为普通 defect 参与排名的设计，由 direct defect 与 conditional local site 分层取代。
- 使用位置等距文本采样作为长文本候选 fingerprint 的设计，由确定性 bottom-k 内容采样取代。
- 直接采用远端 Hirschberg 文本 LCS 的设计已失效；重复字符下无法保持既有 canonical 对齐，当前继续使用有界矩阵实现。

## 必须验收

- 默认和自定义 `fixed_variant_max` 的配置解析、快照及分类边界通过测试。
- 标签后 JSON 的正例和标签不同、尾部后缀、数组重排、值变化负例通过测试。
- bottom-k 集合正确性、中部插入/删除相似度、对称性和排列确定性通过测试。
- LCS 歧义字符串与旧 canonical 映射逐项一致；现有模板、defect identity、性质测试和规模预算不回归。
- 所有 case fixtures 通过 Analyzer 文件入口并验证目标 facts、Session 或缺陷结果。
- 公共产物只存在一套 v0.1.1 Session 与多位点协议。
- debug/release workspace tests、严格 Clippy、格式检查和三二进制 E2E 全部通过。
