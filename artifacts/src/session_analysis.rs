//! 本文件定义 Session 前缀延续分析的稳定磁盘协议。

use serde::{Deserialize, Serialize};

use crate::{OptimizationInsight, SourceLocation};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionAnalysis {
    pub session_record_count: usize,
    pub timelines: Vec<SessionTimeline>,
    pub history_sites: Vec<SessionHistorySite>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionTimeline {
    pub session_id: String,
    pub requests: Vec<SessionRequestReference>,
    pub transitions: Vec<SessionTransition>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionRequestReference {
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captured_at: Option<String>,
    pub input_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionTransition {
    pub id: String,
    pub previous_request_id: String,
    pub current_request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_captured_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_captured_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boundary: Option<SessionBoundaryContext>,
    pub outcome: SessionTransitionOutcome,
}

/// Session 内两条请求可比较时必须完全一致的已知上下文边界。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionBoundaryContext {
    pub endpoint_key: String,
    pub model: String,
    pub context_schema_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_deployment_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kv_namespace: Option<String>,
    pub dialect: String,
    pub adapter_revision: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SessionTransitionOutcome {
    Identical {
        metrics: SessionPrefixMetrics,
    },
    NormalAppend {
        metrics: SessionPrefixMetrics,
    },
    PrefixTruncated {
        metrics: SessionPrefixMetrics,
        divergence: SessionDivergence,
    },
    HistoryChanged {
        metrics: SessionPrefixMetrics,
        divergence: SessionDivergence,
    },
    IncomparableBoundary {
        reason: String,
    },
    AmbiguousOrder {
        reason: String,
    },
    UnanalyzableBoundary {
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionPrefixMetrics {
    pub previous_observable_bytes: usize,
    pub current_observable_bytes: usize,
    pub preserved_prefix_bytes: usize,
    pub prefix_retention_ratio: f64,
    pub invalidated_previous_suffix_bytes: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionDivergence {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_source: Option<SourceLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_source: Option<SourceLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_excerpt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_excerpt: Option<String>,
    pub logical_position: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionHistorySite {
    pub id: String,
    pub kind: SessionHistorySiteKind,
    pub boundary: SessionBoundaryContext,
    pub logical_position: String,
    pub transition_ids: Vec<String>,
    pub occurrence_count: usize,
    pub affected_session_count: usize,
    pub invalidated_previous_suffix_bytes_total: usize,
    pub invalidated_previous_suffix_bytes_min: usize,
    pub invalidated_previous_suffix_bytes_max: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insight: Option<OptimizationInsight>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionHistorySiteKind {
    HistoryChanged,
    PrefixTruncated,
}
