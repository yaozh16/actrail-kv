//! Corpus records preserve complete payloads and make every rejected input explicit.

use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CaptureComparison {
    pub endpoint_key: String,
    pub agent_key: Option<String>,
    pub model_deployment_key: Option<String>,
    pub kv_namespace: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CorpusRecord {
    pub id: String,
    pub captured_at: Option<String>,
    pub source: Option<String>,
    pub session_key: Option<String>,
    pub comparison: CaptureComparison,
    pub payload: Value,
    pub input_line: usize,
}

#[derive(Clone, Debug, Default)]
pub struct AnalysisCorpus {
    pub records: Vec<CorpusRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CorpusSkipReason {
    EmptyLine,
    RecordTooLarge { bytes: usize, limit: usize },
    InvalidJson(String),
    RecordNotObject,
    MissingPayload,
    MissingComparison,
    MissingEndpointKey,
    EmptyEndpointKey,
    InvalidComparisonKey { field: String },
    EmptyComparisonKey { field: String },
    InvalidSessionKey,
    EmptySessionKey,
    PayloadNotObject,
    RecordBudgetExceeded { limit: usize },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorpusSkip {
    pub input_line: usize,
    pub reason: CorpusSkipReason,
}

#[derive(Clone, Debug, Default)]
pub struct CorpusLoadResult {
    pub corpus: AnalysisCorpus,
    pub skipped: Vec<CorpusSkip>,
}
