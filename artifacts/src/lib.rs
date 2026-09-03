//! 本 crate 定义三个二进制之间稳定、可序列化的磁盘数据契约。

mod analysis_result;
mod captured_request;

pub use analysis_result::{
    AnalysisOptionsSnapshot, AnalysisResult, AnalysisRunSummary, ComparisonGroup, ContextDefect,
    DefectFact, DefectFactKind, MismatchPattern, MismatchRegion, MismatchVariant,
    OptimizationInsight, RecoveredStable, RequestTemplate, ScoreBreakdown, SessionEventType,
    SessionReport, SessionSwitchEvent, SkipRecord, SourceLocation, StableSpan, TemplateSlot,
    VariantEvidence, ANALYSIS_SCHEMA_VERSION,
};
pub use captured_request::{CapturedRequest, ComparisonMetadata};
