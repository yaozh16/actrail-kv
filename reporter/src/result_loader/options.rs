//! 本文件校验 analysis 运行参数快照与 Analyzer 的 AnalysisOptions 边界一致。

use actrail_kv_artifacts::AnalysisOptionsSnapshot;
use anyhow::{bail, Result};

use super::common::is_probability;

pub(super) fn validate_options(options: &AnalysisOptionsSnapshot) -> Result<()> {
    if options.top_k == 0 {
        bail!("run.options.top_k must be greater than zero");
    }
    if options.comparison_window_seconds == 0 {
        bail!("run.options.comparison_window_seconds must be greater than zero");
    }
    if options.min_template_members < 2 || options.min_stable_support < 2 {
        bail!("run.options template and stable support minima must be at least two");
    }
    for (name, value) in [
        (
            "stable_span_support_ratio",
            options.stable_span_support_ratio,
        ),
        (
            "text_similarity_threshold",
            options.text_similarity_threshold,
        ),
        (
            "template_compatibility_threshold",
            options.template_compatibility_threshold,
        ),
        (
            "max_dynamic_coverage_ratio",
            options.max_dynamic_coverage_ratio,
        ),
    ] {
        if !is_probability(value) {
            bail!("run.options.{name} must be a finite value between zero and one");
        }
    }
    for (name, value) in [
        ("fixed_variant_max", options.fixed_variant_max),
        ("min_blocked_stable_bytes", options.min_blocked_stable_bytes),
        ("min_exact_anchor_bytes", options.min_exact_anchor_bytes),
        (
            "max_candidates_per_request",
            options.max_candidates_per_request,
        ),
        ("max_projection_units", options.max_projection_units),
        ("max_text_unit_bytes", options.max_text_unit_bytes),
        ("max_payload_bytes", options.max_payload_bytes),
        ("max_alignment_cells", options.max_alignment_cells),
        (
            "max_total_alignment_cells",
            options.max_total_alignment_cells,
        ),
        ("max_records", options.max_records),
    ] {
        if value == 0 {
            bail!("run.options.{name} must be greater than zero");
        }
    }
    Ok(())
}
