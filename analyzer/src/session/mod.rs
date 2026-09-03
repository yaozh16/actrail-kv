//! 会话内 prefix-switch（append/fork/reorder/reset）证据分析。

mod lineage;

pub(crate) use lineage::{analyze_reports, SessionRow};
