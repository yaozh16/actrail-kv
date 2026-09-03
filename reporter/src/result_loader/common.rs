//! 本文件提供 Reporter artifact 校验复用的概率、比较组与来源位置规则。

use actrail_kv_artifacts::{ComparisonGroup, SourceLocation};
use anyhow::{bail, Result};

pub(super) fn validate_source(source: &SourceLocation) -> Result<()> {
    if source.json_path.is_empty() {
        bail!("source JSON path must not be empty");
    }
    match (source.byte_start, source.byte_end) {
        (Some(start), Some(end)) if start <= end => Ok(()),
        (None, None) => Ok(()),
        _ => bail!("source byte offsets must be a valid start/end pair"),
    }
}

pub(super) fn validate_group(group: &ComparisonGroup) -> Result<()> {
    if group.time_window_key.is_empty()
        || group.endpoint_key.is_empty()
        || group.model.is_empty()
        || group.context_schema_key.is_empty()
    {
        bail!("comparison group required keys must not be empty");
    }
    Ok(())
}

pub(super) fn is_probability(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

pub(super) fn floats_match(actual: f64, expected: f64) -> bool {
    let tolerance = f64::EPSILON * expected.abs().max(1.0) * 8.0;
    actual.is_finite() && (actual - expected).abs() <= tolerance
}
