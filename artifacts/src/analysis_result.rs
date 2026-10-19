//! 本文件定义统一 P1/X/P2 缺陷模型的 analysis.json 磁盘协议。

use serde::{Deserialize, Serialize};

mod cache_metric;
mod prefix_reuse;
mod run_summary;
mod session;

pub use cache_metric::{CacheMetricBasis, KvCacheMetric};
pub use prefix_reuse::{
    PrefixReuseComposition, PrefixReuseRequest, PrefixReuseSession, PrefixReuseSummary,
};
pub use run_summary::{AnalysisOptionsSnapshot, AnalysisRunSummary, SkipRecord};
pub use session::{SessionEventType, SessionReport, SessionSwitchEvent};

pub const ANALYSIS_SCHEMA_VERSION: &str = "0.1.0";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisResult {
    pub run: AnalysisRunSummary,
    pub templates: Vec<RequestTemplate>,
    pub defects: Vec<ContextDefect>,
    /// 按优先级排列的 defect ID；排名由数组位置表达。
    pub top_k: Vec<String>,
    /// 会话内 prefix-switch 证据；旧版文件缺省为空。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub session_reports: Vec<SessionReport>,
    /// 每请求 KV 命中率指标（真实 usage 优先，缺失时按上下文估算）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cache_metrics: Vec<KvCacheMetric>,
    /// 会话内相邻请求对的结构复用率汇总；没有会话对时缺省。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix_reuse: Option<PrefixReuseSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonGroup {
    pub time_window_key: String,
    pub endpoint_key: String,
    pub model: String,
    pub context_schema_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_deployment_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kv_namespace: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestTemplate {
    pub id: String,
    pub comparison_group: ComparisonGroup,
    pub member_request_ids: Vec<String>,
    pub medoid_request_id: String,
    pub cohesion: f64,
    pub stable_spans: Vec<StableSpan>,
    pub slots: Vec<TemplateSlot>,
    /// 每模板一棵前缀树的可视化视图模型（供 report 的 Prefix Tree 面板使用）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix_view: Option<TemplatePrefixView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplatePrefixView {
    pub node_count: usize,
    pub truncated: bool,
    pub nodes: Vec<PrefixNode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum PrefixNodeKind {
    Stable,
    Dynamic,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrefixNode {
    pub id: String,
    pub kind: PrefixNodeKind,
    pub source: SourceLocation,
    pub utf8_bytes: usize,
    pub support_count: usize,
    pub support_ratio: f64,
    pub excerpt: String,
    /// 在该节点（片段）仍然存活的成员请求 id；用于下钻时展示存活/衰减明细。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub member_request_ids: Vec<String>,
    /// 稳定节点的字节内容样本（供详情面板展示，不进入树节点）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_sample: Option<String>,
    /// 对齐序列层的下一个片段（P → X → P2 链）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence_child: Option<String>,
    /// 内部字节层的子节点 id。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub internal_children: Vec<String>,
    /// dynamic 节点的取值列表；stable 节点为 None。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variants: Option<Vec<PrefixVariant>>,
    /// 与该 X 区重叠的缺陷 id（与缺陷口径一致）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub defect_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrefixVariant {
    pub excerpt: String,
    pub count: usize,
    /// 计数是否为精确统计（无法按成员精确对齐时为 false）。
    pub count_known: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StableSpan {
    pub source: SourceLocation,
    pub utf8_bytes: usize,
    pub support_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excerpt: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateSlot {
    pub source: SourceLocation,
    pub member_count: usize,
    pub distinct_variant_count: usize,
    pub confidence: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextDefect {
    pub id: String,
    pub comparison_group: ComparisonGroup,
    pub template_id: String,
    pub mismatch: MismatchRegion,
    pub recovered_stable: RecoveredStable,
    pub actual_prefix_bytes: usize,
    pub potential_prefix_bytes: usize,
    pub blocked_stable_bytes: usize,
    pub comparable_count: usize,
    pub affected_count: usize,
    pub confidence: f64,
    pub score: ScoreBreakdown,
    pub insights: Vec<OptimizationInsight>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MismatchRegion {
    pub pattern: MismatchPattern,
    pub variants: Vec<MismatchVariant>,
    pub facts: Vec<DefectFact>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MismatchPattern {
    ValueMismatch,
    InsertionDeletion,
    Reorder,
    Mixed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MismatchVariant {
    pub fingerprint: String,
    pub member_request_ids: Vec<String>,
    pub representative: VariantEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariantEvidence {
    pub request_id: String,
    pub sources: Vec<SourceLocation>,
    pub utf8_bytes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excerpt: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveredStable {
    pub sources: Vec<SourceLocation>,
    pub utf8_bytes: usize,
    pub support_count: usize,
    pub excerpt: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefectFact {
    pub kind: DefectFactKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefectFactKind {
    ContentVariation,
    FixedVariants,
    StructuredDataEquivalent,
    InsertionDeletion,
    Reorder,
    Scope,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptimizationInsight {
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceLocation {
    pub json_path: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub logical_scope: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byte_start: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byte_end: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScoreBreakdown {
    pub blocked_stable_bytes: usize,
    pub affected_count: usize,
    pub confidence: f64,
    pub score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unified_defect_round_trips_with_snake_case_enums() {
        let result = fixture();
        let encoded = serde_json::to_string(&result).expect("serialize analysis result");
        assert!(encoded.contains("\"pattern\":\"mixed\""));
        assert!(encoded.contains("\"kind\":\"structured_data_equivalent\""));
        assert_eq!(
            serde_json::from_str::<AnalysisResult>(&encoded).expect("deserialize analysis result"),
            result
        );
    }

    #[test]
    fn missing_variant_can_have_no_source_or_excerpt() {
        let mut result = fixture();
        let evidence = &mut result.defects[0].mismatch.variants[1].representative;
        evidence.sources.clear();
        evidence.excerpt = None;
        evidence.utf8_bytes = 0;
        let encoded = serde_json::to_string(&result).expect("serialize analysis result");
        let decoded: AnalysisResult = serde_json::from_str(&encoded).expect("deserialize result");
        assert!(decoded.defects[0].mismatch.variants[1]
            .representative
            .sources
            .is_empty());
    }

    #[test]
    fn all_mismatch_patterns_have_stable_snake_case_names() {
        for (pattern, expected) in [
            (MismatchPattern::ValueMismatch, "\"value_mismatch\""),
            (MismatchPattern::InsertionDeletion, "\"insertion_deletion\""),
            (MismatchPattern::Reorder, "\"reorder\""),
            (MismatchPattern::Mixed, "\"mixed\""),
        ] {
            assert_eq!(
                serde_json::to_string(&pattern).expect("serialize"),
                expected
            );
        }
    }

    fn fixture() -> AnalysisResult {
        let group = ComparisonGroup {
            time_window_key: "2026-09-02T08:00Z/1h".into(),
            endpoint_key: "primary".into(),
            model: "model-a".into(),
            context_schema_key: "openai-chat/v1".into(),
            agent_key: Some("coding".into()),
            model_deployment_key: Some("deployment-a".into()),
            kv_namespace: None,
        };
        let source = SourceLocation {
            json_path: "$.messages[0].content".into(),
            logical_scope: vec!["messages".into(), "system".into()],
            unit_index: Some(1),
            byte_start: Some(4),
            byte_end: Some(8),
            role: Some("system".into()),
        };
        let score = ScoreBreakdown {
            blocked_stable_bytes: 128,
            affected_count: 2,
            confidence: 0.75,
            score: 192.0,
        };
        AnalysisResult {
            run: AnalysisRunSummary {
                schema_version: ANALYSIS_SCHEMA_VERSION.into(),
                input_records: 4,
                analyzed_records: 4,
                skipped_records: vec![],
                options: options(),
                limitations: vec!["structural proxy only".into()],
            },
            templates: vec![RequestTemplate {
                id: "template-a".into(),
                comparison_group: group.clone(),
                member_request_ids: vec!["a".into(), "b".into(), "c".into(), "d".into()],
                medoid_request_id: "a".into(),
                cohesion: 0.9,
                stable_spans: vec![],
                slots: vec![],
                prefix_view: None,
            }],
            defects: vec![ContextDefect {
                id: "defect-a".into(),
                comparison_group: group,
                template_id: "template-a".into(),
                mismatch: MismatchRegion {
                    pattern: MismatchPattern::Mixed,
                    variants: vec![
                        variant("variant-a", &["a", "b"], "a", Some("north"), &source),
                        variant("variant-b", &["c", "d"], "c", Some("south"), &source),
                    ],
                    facts: vec![DefectFact {
                        kind: DefectFactKind::StructuredDataEquivalent,
                        detail: Some("JSON objects are strictly equivalent".into()),
                    }],
                },
                recovered_stable: RecoveredStable {
                    sources: vec![source],
                    utf8_bytes: 128,
                    support_count: 4,
                    excerpt: "stable P2".into(),
                },
                actual_prefix_bytes: 16,
                potential_prefix_bytes: 144,
                blocked_stable_bytes: 128,
                comparable_count: 4,
                affected_count: 2,
                confidence: 0.75,
                score,
                insights: vec![OptimizationInsight {
                    summary: "统一 X 的生成方式".into(),
                    detail: Some("在语义允许时将稳定 P2 前移".into()),
                }],
            }],
            top_k: vec!["defect-a".into()],
            session_reports: vec![],
            cache_metrics: vec![],
            prefix_reuse: None,
        }
    }

    fn variant(
        fingerprint: &str,
        members: &[&str],
        representative: &str,
        excerpt: Option<&str>,
        source: &SourceLocation,
    ) -> MismatchVariant {
        MismatchVariant {
            fingerprint: fingerprint.into(),
            member_request_ids: members.iter().map(|value| (*value).into()).collect(),
            representative: VariantEvidence {
                request_id: representative.into(),
                sources: vec![source.clone()],
                utf8_bytes: excerpt.map_or(0, str::len),
                excerpt: excerpt.map(str::to_owned),
            },
        }
    }

    fn options() -> AnalysisOptionsSnapshot {
        AnalysisOptionsSnapshot {
            top_k: 20,
            comparison_window_seconds: 3_600,
            min_template_members: 3,
            stable_span_support_ratio: 0.8,
            min_stable_support: 3,
            fixed_variant_max: 3,
            min_blocked_stable_bytes: 64,
            min_exact_anchor_bytes: 24,
            text_similarity_threshold: 0.8,
            template_compatibility_threshold: 0.68,
            max_dynamic_coverage_ratio: 0.35,
            max_candidates_per_request: 128,
            max_projection_units: 512,
            max_text_unit_bytes: 1_048_576,
            max_payload_bytes: 8_388_608,
            max_alignment_cells: 2_000_000,
            max_total_alignment_cells: 64_000_000,
            max_records: 1_000_000,
            prefix_view_max_nodes: 5_000,
            prefix_view_excerpt_bytes: 96,
        }
    }
}
