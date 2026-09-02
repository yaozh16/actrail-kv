//! 本文件定义 analysis.json 的运行摘要、模板、Finding、证据与排名格式。

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisResult {
    pub run: AnalysisRunSummary,
    pub templates: Vec<RequestTemplate>,
    pub findings: Vec<Finding>,
    pub top_k: Vec<TopKEntry>,
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
    pub min_template_members: usize,
    pub stable_span_support_ratio: f64,
    pub min_stable_support: usize,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkipRecord {
    pub record_index: usize,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestTemplate {
    pub id: String,
    pub dialect: String,
    pub model: String,
    pub adapter_revision: String,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingCause {
    EarlyVariableContent,
    InlineDynamicSlot,
    DynamicBlockBeforeStable,
    ToolOrderDrift,
    ToolDefinitionDrift,
    SystemPromptDrift,
    NonAppendOnlyHistory,
    ModelVisibleFormatDrift,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub id: String,
    pub template_id: String,
    pub cause: FindingCause,
    pub source: SourceLocation,
    pub actual_prefix_bytes: usize,
    pub potential_prefix_bytes: usize,
    pub blocked_stable_bytes: usize,
    pub affected_count: usize,
    pub confidence: f64,
    pub score: ScoreBreakdown,
    pub evidence: Evidence,
    pub counterfactual: String,
    pub recommendation: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceLocation {
    pub json_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byte_start: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byte_end: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub representative_request_ids: Vec<String>,
    pub divergent_excerpts: Vec<String>,
    pub blocked_stable_excerpt: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScoreBreakdown {
    pub blocked_stable_bytes: usize,
    pub affected_count: usize,
    pub confidence: f64,
    pub score: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TopKEntry {
    pub rank: usize,
    pub finding_id: String,
    pub score: ScoreBreakdown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_analysis_result_round_trips_with_snake_case_cause() {
        let source = SourceLocation {
            json_path: "$.messages[0].content".to_owned(),
            unit_index: Some(3),
            byte_start: Some(6),
            byte_end: Some(12),
            role: Some("system".to_owned()),
        };
        let score = ScoreBreakdown {
            blocked_stable_bytes: 128,
            affected_count: 4,
            confidence: 0.75,
            score: 384.0,
        };
        let result = AnalysisResult {
            run: AnalysisRunSummary {
                schema_version: "0.1.0".to_owned(),
                input_records: 5,
                analyzed_records: 5,
                skipped_records: Vec::new(),
                options: options(),
                limitations: vec!["structural proxy only".to_owned()],
            },
            templates: vec![RequestTemplate {
                id: "tpl-1".to_owned(),
                dialect: "openai_chat".to_owned(),
                model: "example".to_owned(),
                adapter_revision: "1".to_owned(),
                member_request_ids: vec![
                    "request-a".to_owned(),
                    "request-b".to_owned(),
                    "request-c".to_owned(),
                    "request-d".to_owned(),
                    "request-e".to_owned(),
                ],
                medoid_request_id: "request-a".to_owned(),
                cohesion: 0.9,
                stable_spans: vec![StableSpan {
                    source: source.clone(),
                    utf8_bytes: 128,
                    support_count: 5,
                    excerpt: Some("stable".to_owned()),
                }],
                slots: vec![TemplateSlot {
                    source: source.clone(),
                    member_count: 5,
                    distinct_variant_count: 2,
                    confidence: 0.9,
                }],
            }],
            findings: vec![Finding {
                id: "finding-1".to_owned(),
                template_id: "tpl-1".to_owned(),
                cause: FindingCause::InlineDynamicSlot,
                source,
                actual_prefix_bytes: 6,
                potential_prefix_bytes: 134,
                blocked_stable_bytes: 128,
                affected_count: 4,
                confidence: 0.75,
                score: score.clone(),
                evidence: Evidence {
                    representative_request_ids: vec![
                        "request-a".to_owned(),
                        "request-b".to_owned(),
                    ],
                    divergent_excerpts: vec!["a".to_owned(), "b".to_owned()],
                    blocked_stable_excerpt: "stable".to_owned(),
                },
                counterfactual: "move the variable suffix".to_owned(),
                recommendation: "keep stable content first".to_owned(),
            }],
            top_k: vec![TopKEntry {
                rank: 1,
                finding_id: "finding-1".to_owned(),
                score,
            }],
        };

        let encoded = serde_json::to_string(&result).expect("serialize analysis result");
        assert!(encoded.contains("\"cause\":\"inline_dynamic_slot\""));
        assert_eq!(
            serde_json::from_str::<AnalysisResult>(&encoded).expect("deserialize analysis result"),
            result
        );
    }

    fn options() -> AnalysisOptionsSnapshot {
        AnalysisOptionsSnapshot {
            top_k: 100,
            min_template_members: 3,
            stable_span_support_ratio: 0.8,
            min_stable_support: 3,
            min_blocked_stable_bytes: 64,
            min_exact_anchor_bytes: 24,
            text_similarity_threshold: 0.8,
            template_compatibility_threshold: 0.68,
            max_dynamic_coverage_ratio: 0.35,
            max_candidates_per_request: 128,
            max_projection_units: 512,
            max_text_unit_bytes: 1024 * 1024,
            max_payload_bytes: 8 * 1024 * 1024,
            max_alignment_cells: 2_000_000,
            max_total_alignment_cells: 64_000_000,
            max_records: 1_000_000,
        }
    }
}
