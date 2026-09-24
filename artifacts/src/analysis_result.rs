//! 本文件定义统一 P1/X/P2 缺陷模型的 analysis.json 磁盘协议。

use serde::{Deserialize, Serialize};

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
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum CacheMetricBasis {
    /// 来自上游响应 usage 的真实计数。
    Reported,
    /// 上游未返回 usage，按与上一次请求的上下文公共前缀估算。
    Estimated,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KvCacheMetric {
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_key: Option<String>,
    /// 会话内序号（无 session 时按输入顺序）。
    pub sequence: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captured_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub miss_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u32>,
    /// 命中率 = cached / prompt（0..=1）。
    pub hit_rate: f64,
    /// 相对同一会话上一条请求的命中率变化。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hit_rate_delta: Option<f64>,
    pub basis: CacheMetricBasis,
    /// 估算口径下与上一条请求的公共前缀字节。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_lcp_bytes: Option<usize>,
    pub payload_bytes: usize,
    /// 首 token 时延（毫秒）；未上报时缺省。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttft_ms: Option<u64>,
    /// 总耗时（wall time，毫秒）；未上报时缺省。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_ms: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum SessionEventType {
    Append,
    Fork,
    Reorder,
    Reset,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionSwitchEvent {
    pub event_type: SessionEventType,
    pub prev_request_id: String,
    pub next_request_id: String,
    pub lcp_units: usize,
    pub lcp_bytes: usize,
    /// 相邻前一对的公共前缀字节（首事件为 0）。
    pub previous_pair_lcp_bytes: usize,
    /// 前缀收缩字节：previous_pair_lcp 与当前 lcp 的差（无收缩为 0）。
    pub prefix_cut_bytes: usize,
    pub next_total_bytes: usize,
    pub recomputed_bytes: usize,
    pub stable_after_switch_bytes: usize,
    /// 下一条请求内容性单位的稳定块 run-length 摘要（如 4 个相同块为 x4）。
    pub next_block_runs: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionReport {
    pub session_key: String,
    pub endpoint_key: String,
    pub model: String,
    pub request_count: usize,
    pub append_count: usize,
    pub fork_count: usize,
    pub reorder_count: usize,
    pub reset_count: usize,
    pub total_recomputed_bytes: usize,
    pub total_stable_after_switch_bytes: usize,
    pub total_prefix_cut_bytes: usize,
    pub avg_prefix_cut_bytes: usize,
    pub events: Vec<SessionSwitchEvent>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisRunSummary {
    pub schema_version: String,
    pub input_records: usize,
    pub analyzed_records: usize,
    pub skipped_records: Vec<SkipRecord>,
    pub options: AnalysisOptionsSnapshot,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisOptionsSnapshot {
    pub top_k: usize,
    pub comparison_window_seconds: u64,
    pub min_template_members: usize,
    pub stable_span_support_ratio: f64,
    pub min_stable_support: usize,
    #[serde(default = "default_fixed_variant_max")]
    pub fixed_variant_max: usize,
    pub min_blocked_stable_bytes: usize,
    pub min_exact_anchor_bytes: usize,
    pub text_similarity_threshold: f64,
    pub template_compatibility_threshold: f64,
    pub max_dynamic_coverage_ratio: f64,
    pub max_candidates_per_request: usize,
    pub max_projection_units: usize,
    pub max_text_unit_bytes: usize,
    pub max_payload_bytes: usize,
    pub max_alignment_cells: usize,
    pub max_total_alignment_cells: usize,
    pub max_records: usize,
}

fn default_fixed_variant_max() -> usize {
    3
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkipRecord {
    pub record_index: usize,
    pub reason: String,
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
        }
    }
}
