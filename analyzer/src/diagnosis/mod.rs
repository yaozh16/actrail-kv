//! 本模块把已投影的模型可见请求归因为可解释的上下文前缀破坏机会。

mod engine;
mod support;

use serde::{Deserialize, Serialize};

use crate::discovery::template::RequestTemplate;
use crate::model::projection::{CacheSequence, CacheUnitKind};

pub use engine::diagnose_template;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitRole {
    SystemPrompt,
    ToolDefinition,
    Conversation,
    ModelVisibleText,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub json_path: String,
    pub utf8_range: Option<(usize, usize)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticUnit {
    pub role: UnitRole,
    pub content: String,
    pub source: SourceLocation,
    pub stable_identity: Option<String>,
    pub ordinal: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticRequest {
    pub request_id: String,
    pub units: Vec<DiagnosticUnit>,
}

impl From<&CacheSequence> for DiagnosticRequest {
    fn from(sequence: &CacheSequence) -> Self {
        Self {
            request_id: sequence.request_id.clone(),
            units: sequence
                .units
                .iter()
                .enumerate()
                .map(|(unit_index, unit)| {
                    let role = match unit.kind {
                        CacheUnitKind::ToolDefinition => UnitRole::ToolDefinition,
                        CacheUnitKind::Role if unit.source.message_index.is_some() => {
                            UnitRole::Conversation
                        }
                        CacheUnitKind::VisibleText
                            if unit.source.role.as_deref() == Some("system") =>
                        {
                            UnitRole::SystemPrompt
                        }
                        CacheUnitKind::VisibleText if unit.source.message_index.is_some() => {
                            UnitRole::Conversation
                        }
                        CacheUnitKind::VisibleText => UnitRole::ModelVisibleText,
                        _ => UnitRole::Other,
                    };
                    DiagnosticUnit {
                        role,
                        content: unit.content.clone(),
                        source: SourceLocation {
                            json_path: unit.source.json_path.clone(),
                            utf8_range: unit
                                .source
                                .utf8_bytes
                                .as_ref()
                                .map(|range| (range.start, range.end)),
                        },
                        stable_identity: unit
                            .tool_identity
                            .clone()
                            .or_else(|| Some(unit.alignment_key.clone())),
                        ordinal: unit.source.message_index.or(Some(unit_index)),
                    }
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticTemplate {
    pub template_id: String,
    pub members: Vec<DiagnosticRequest>,
    pub cohesion: f64,
    pub projection_reliability: f64,
}

impl From<&RequestTemplate> for DiagnosticTemplate {
    fn from(template: &RequestTemplate) -> Self {
        Self {
            template_id: template.id.clone(),
            members: template
                .members
                .iter()
                .map(DiagnosticRequest::from)
                .collect(),
            cohesion: template.cohesion,
            projection_reliability: template.projection_reliability,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cause {
    EarlyVolatileContent,
    InlineDynamicSlot,
    DynamicBlockBeforeStatic,
    ToolOrderDrift,
    ToolDefinitionDrift,
    PromptMicroDrift,
    NonAppendHistory,
    ModelVisibleJsonFormattingDrift,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DiagnosisOptions {
    pub min_stable_support: usize,
    pub stable_support_rate: f64,
    pub min_blocked_bytes: usize,
    pub min_anchor_bytes: usize,
    pub local_text_similarity: f64,
}

impl Default for DiagnosisOptions {
    fn default() -> Self {
        Self {
            min_stable_support: 3,
            stable_support_rate: 0.8,
            min_blocked_bytes: 64,
            min_anchor_bytes: 24,
            local_text_similarity: 0.8,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DiagnosisObservation {
    pub template_id: String,
    pub template_member_count: usize,
    pub member_id: String,
    pub cause: Cause,
    pub normalized_position: String,
    pub stable_anchor: String,
    pub actual_prefix_bytes: usize,
    pub potential_prefix_bytes: usize,
    pub blocked_stable_bytes: usize,
    pub confidence: f64,
    pub source: SourceLocation,
    pub evidence: String,
    pub counterfactual: String,
    pub recommendation: String,
    pub variant_fingerprint: String,
}
