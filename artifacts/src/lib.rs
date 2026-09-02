//! 本 crate 定义三个二进制之间稳定、可序列化的磁盘数据契约。

mod analysis_result;
mod captured_request;

pub use analysis_result::{
    AnalysisOptionsSnapshot, AnalysisResult, AnalysisRunSummary, Evidence, Finding, FindingCause,
    RequestTemplate, ScoreBreakdown, SkipRecord, SourceLocation, StableSpan, TemplateSlot,
    TopKEntry,
};
pub use captured_request::CapturedRequest;
