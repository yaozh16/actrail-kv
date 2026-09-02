//! Projection types retain observable UTF-8 content and exact JSON source locations.

use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComparisonDomain {
    pub dialect: String,
    pub model: String,
    pub adapter_revision: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CacheUnitKind {
    Structural,
    Role,
    VisibleText,
    ToolDefinition,
    ToolCall,
    ToolResult,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceLocation {
    pub json_path: String,
    pub utf8_bytes: Option<Range<usize>>,
    pub message_index: Option<usize>,
    pub role: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CacheUnit {
    pub kind: CacheUnitKind,
    pub alignment_key: String,
    pub content: String,
    pub source: SourceLocation,
    pub tool_identity: Option<String>,
}

impl CacheUnit {
    pub fn observable_bytes(&self) -> usize {
        self.content.len()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CacheSequence {
    pub request_id: String,
    pub domain: ComparisonDomain,
    pub units: Vec<CacheUnit>,
    pub projection_reliability_millis: u16,
}

impl CacheSequence {
    pub fn projection_reliability(&self) -> f64 {
        f64::from(self.projection_reliability_millis) / 1000.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectionSkipReason {
    PayloadNotObject,
    MissingModel,
    MissingMessages,
    MessagesNotArray,
    MessageNotObject {
        index: usize,
    },
    UnitBudgetExceeded {
        limit: usize,
    },
    TextBudgetExceeded {
        path: String,
        bytes: usize,
        limit: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectionSkip {
    pub request_id: String,
    pub reason: ProjectionSkipReason,
}
