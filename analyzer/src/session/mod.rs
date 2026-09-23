//! 会话内 prefix-switch（append/fork/reorder/reset）证据分析。

mod lineage;

pub(crate) use lineage::{
    analyze_reports, request_potential_by_request, summarize_prefix_reuse, SessionRow,
};
