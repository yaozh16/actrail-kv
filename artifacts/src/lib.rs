//! 本 crate 定义三个二进制之间稳定、可序列化的磁盘数据契约。

mod analysis_result;
mod captured_request;
mod session_analysis;

pub use analysis_result::{
    AnalysisOptionsSnapshot, AnalysisResult, AnalysisRunSummary, ComparisonGroup,
    ConditionalLocalSite, ContextDefect, DefectFact, DefectFactKind, MismatchPattern,
    MismatchRegion, MismatchVariant, OptimizationInsight, RecoveredStable, RequestTemplate,
    ScoreBreakdown, SkipRecord, SourceLocation, StableSpan, TemplateSlot, VariantEvidence,
    ANALYSIS_SCHEMA_VERSION,
};
pub use captured_request::{CapturedRequest, ComparisonMetadata};
pub use session_analysis::{
    SessionAnalysis, SessionBoundaryContext, SessionDivergence, SessionHistorySite,
    SessionHistorySiteKind, SessionPrefixMetrics, SessionRequestReference, SessionTimeline,
    SessionTransition, SessionTransitionOutcome,
};
