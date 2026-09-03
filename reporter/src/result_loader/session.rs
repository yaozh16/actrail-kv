//! 本文件校验 Session 时间线的相邻引用、前缀指标、分类与历史位点聚合。

use std::collections::{BTreeMap, BTreeSet};

use actrail_kv_artifacts::{
    SessionAnalysis, SessionBoundaryContext, SessionHistorySiteKind, SessionPrefixMetrics,
    SessionTransitionOutcome,
};
use anyhow::{bail, Result};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use super::common::{floats_match, validate_source};

struct HistoryTransition<'a> {
    session_id: &'a str,
    kind: SessionHistorySiteKind,
    boundary: &'a SessionBoundaryContext,
    logical_position: &'a str,
    invalidated_bytes: usize,
}

pub(super) fn validate_session_analysis(analysis: &SessionAnalysis) -> Result<()> {
    let mut session_ids = BTreeSet::new();
    let mut request_ids = BTreeSet::new();
    let mut transition_ids = BTreeSet::new();
    let mut history_transitions = BTreeMap::new();
    let mut record_count = 0usize;

    for timeline in &analysis.timelines {
        if timeline.session_id.is_empty() || !session_ids.insert(&timeline.session_id) {
            bail!("Session IDs must be non-empty and unique");
        }
        record_count = record_count
            .checked_add(timeline.requests.len())
            .ok_or_else(|| anyhow::anyhow!("Session record count overflow"))?;
        for request in &timeline.requests {
            if request.request_id.is_empty() || !request_ids.insert(&request.request_id) {
                bail!("Session request IDs must be non-empty and globally unique");
            }
        }
        if timeline.transitions.len() != timeline.requests.len().saturating_sub(1) {
            bail!("Session transitions must connect every adjacent request exactly once");
        }
        let mut timestamp_counts = BTreeMap::new();
        for request in &timeline.requests {
            if let Some(timestamp) = parsed_timestamp(request.captured_at.as_deref()) {
                *timestamp_counts.entry(timestamp).or_insert(0usize) += 1;
            }
        }
        for (index, transition) in timeline.transitions.iter().enumerate() {
            if transition.id.is_empty() || !transition_ids.insert(&transition.id) {
                bail!("Session transition IDs must be non-empty and globally unique");
            }
            let previous = &timeline.requests[index];
            let current = &timeline.requests[index + 1];
            if transition.previous_request_id != previous.request_id
                || transition.current_request_id != current.request_id
                || transition.previous_captured_at != previous.captured_at
                || transition.current_captured_at != current.captured_at
            {
                bail!("Session transition does not reference its adjacent requests");
            }
            if request_order(previous, current) == std::cmp::Ordering::Greater {
                bail!("Session requests are not ordered by captured time and input line");
            }
            validate_outcome(
                transition,
                &timeline.session_id,
                parsed_timestamp(previous.captured_at.as_deref()).is_none()
                    || parsed_timestamp(current.captured_at.as_deref()).is_none(),
                [previous, current].iter().any(|request| {
                    parsed_timestamp(request.captured_at.as_deref())
                        .is_some_and(|timestamp| timestamp_counts[&timestamp] > 1)
                }),
                &mut history_transitions,
            )?;
        }
    }
    if record_count != analysis.session_record_count {
        bail!("session_record_count does not equal timeline request count");
    }
    validate_history_sites(analysis, &history_transitions)
}

fn validate_outcome<'a>(
    transition: &'a actrail_kv_artifacts::SessionTransition,
    session_id: &'a str,
    has_missing_time: bool,
    has_ambiguous_time: bool,
    history: &mut BTreeMap<&'a str, HistoryTransition<'a>>,
) -> Result<()> {
    let comparable = matches!(
        transition.outcome,
        SessionTransitionOutcome::Identical { .. }
            | SessionTransitionOutcome::NormalAppend { .. }
            | SessionTransitionOutcome::PrefixTruncated { .. }
            | SessionTransitionOutcome::HistoryChanged { .. }
    );
    if transition.boundary.is_some() != comparable {
        bail!("only comparable Session transitions must carry boundary context");
    }
    if let Some(boundary) = &transition.boundary {
        validate_boundary(boundary)?;
    }
    if has_missing_time
        && !matches!(
            transition.outcome,
            SessionTransitionOutcome::UnanalyzableBoundary { .. }
        )
    {
        bail!("a Session transition with missing capture time must be unanalyzable");
    }
    if !has_missing_time
        && has_ambiguous_time
        && !matches!(
            transition.outcome,
            SessionTransitionOutcome::AmbiguousOrder { .. }
        )
    {
        bail!("a Session transition touching a repeated capture time must be ambiguous");
    }
    if !has_ambiguous_time
        && matches!(
            transition.outcome,
            SessionTransitionOutcome::AmbiguousOrder { .. }
        )
    {
        bail!("an ambiguous Session transition must touch a repeated capture time");
    }
    match &transition.outcome {
        SessionTransitionOutcome::Identical { metrics } => {
            validate_metrics(metrics)?;
            if metrics.previous_observable_bytes != metrics.current_observable_bytes
                || metrics.preserved_prefix_bytes != metrics.previous_observable_bytes
            {
                bail!("identical transition metrics are inconsistent");
            }
        }
        SessionTransitionOutcome::NormalAppend { metrics } => {
            validate_metrics(metrics)?;
            if metrics.preserved_prefix_bytes != metrics.previous_observable_bytes
                || metrics.current_observable_bytes <= metrics.previous_observable_bytes
            {
                bail!("normal append transition metrics are inconsistent");
            }
        }
        SessionTransitionOutcome::PrefixTruncated {
            metrics,
            divergence,
        } => {
            validate_metrics(metrics)?;
            validate_divergence(divergence)?;
            if metrics.preserved_prefix_bytes != metrics.current_observable_bytes
                || metrics.current_observable_bytes >= metrics.previous_observable_bytes
            {
                bail!("prefix truncated transition metrics are inconsistent");
            }
            history.insert(
                &transition.id,
                HistoryTransition {
                    session_id,
                    kind: SessionHistorySiteKind::PrefixTruncated,
                    boundary: transition.boundary.as_ref().expect("validated boundary"),
                    logical_position: &divergence.logical_position,
                    invalidated_bytes: metrics.invalidated_previous_suffix_bytes,
                },
            );
        }
        SessionTransitionOutcome::HistoryChanged {
            metrics,
            divergence,
        } => {
            validate_metrics(metrics)?;
            validate_divergence(divergence)?;
            if metrics.preserved_prefix_bytes >= metrics.previous_observable_bytes
                || metrics.preserved_prefix_bytes >= metrics.current_observable_bytes
            {
                bail!("history changed transition must diverge before both request ends");
            }
            history.insert(
                &transition.id,
                HistoryTransition {
                    session_id,
                    kind: SessionHistorySiteKind::HistoryChanged,
                    boundary: transition.boundary.as_ref().expect("validated boundary"),
                    logical_position: &divergence.logical_position,
                    invalidated_bytes: metrics.invalidated_previous_suffix_bytes,
                },
            );
        }
        SessionTransitionOutcome::IncomparableBoundary { reason }
        | SessionTransitionOutcome::AmbiguousOrder { reason }
        | SessionTransitionOutcome::UnanalyzableBoundary { reason } => {
            if reason.is_empty() {
                bail!("Session boundary reason must not be empty");
            }
        }
    }
    Ok(())
}

fn validate_metrics(metrics: &SessionPrefixMetrics) -> Result<()> {
    if metrics.preserved_prefix_bytes > metrics.previous_observable_bytes
        || metrics.preserved_prefix_bytes > metrics.current_observable_bytes
    {
        bail!("preserved Session prefix exceeds an observable request length");
    }
    if metrics.invalidated_previous_suffix_bytes
        != metrics.previous_observable_bytes - metrics.preserved_prefix_bytes
    {
        bail!("invalidated Session suffix does not match prefix metrics");
    }
    let expected = if metrics.previous_observable_bytes == 0 {
        1.0
    } else {
        metrics.preserved_prefix_bytes as f64 / metrics.previous_observable_bytes as f64
    };
    if !floats_match(metrics.prefix_retention_ratio, expected) {
        bail!("Session prefix retention ratio does not match byte metrics");
    }
    Ok(())
}

fn validate_divergence(divergence: &actrail_kv_artifacts::SessionDivergence) -> Result<()> {
    if divergence.logical_position.is_empty() {
        bail!("Session divergence logical position must not be empty");
    }
    if let Some(source) = &divergence.previous_source {
        validate_source(source)?;
    }
    if let Some(source) = &divergence.current_source {
        validate_source(source)?;
    }
    Ok(())
}

fn validate_history_sites(
    analysis: &SessionAnalysis,
    transitions: &BTreeMap<&str, HistoryTransition<'_>>,
) -> Result<()> {
    let mut site_ids = BTreeSet::new();
    let mut referenced_transitions = BTreeSet::new();
    for site in &analysis.history_sites {
        if site.id.is_empty() || !site_ids.insert(&site.id) || site.logical_position.is_empty() {
            bail!("history site IDs must be unique and IDs/positions must be non-empty");
        }
        if site.transition_ids.is_empty()
            || site.occurrence_count != site.transition_ids.len()
            || site.insight.is_some() != (site.occurrence_count >= 2)
        {
            bail!("history site occurrence and insight fields are inconsistent");
        }
        validate_boundary(&site.boundary)?;
        let mut sessions = BTreeSet::new();
        let mut total = 0usize;
        let mut minimum = usize::MAX;
        let mut maximum = 0usize;
        for transition_id in &site.transition_ids {
            if !referenced_transitions.insert(transition_id) {
                bail!("a history transition is referenced by more than one site");
            }
            let transition = transitions.get(transition_id.as_str()).ok_or_else(|| {
                anyhow::anyhow!("history site references a non-history transition")
            })?;
            if transition.logical_position != site.logical_position {
                bail!("history site and transition logical positions do not match");
            }
            if transition.kind != site.kind || transition.boundary != &site.boundary {
                bail!("history site kind and boundary must match every referenced transition");
            }
            sessions.insert(transition.session_id);
            total = total
                .checked_add(transition.invalidated_bytes)
                .ok_or_else(|| anyhow::anyhow!("history site byte total overflow"))?;
            minimum = minimum.min(transition.invalidated_bytes);
            maximum = maximum.max(transition.invalidated_bytes);
        }
        if site.affected_session_count != sessions.len()
            || site.invalidated_previous_suffix_bytes_total != total
            || site.invalidated_previous_suffix_bytes_min != minimum
            || site.invalidated_previous_suffix_bytes_max != maximum
        {
            bail!("history site aggregate metrics do not match referenced transitions");
        }
    }
    if referenced_transitions.len() != transitions.len() {
        bail!("every history transition must belong to exactly one history site");
    }
    Ok(())
}

fn validate_boundary(boundary: &SessionBoundaryContext) -> Result<()> {
    if boundary.endpoint_key.is_empty()
        || boundary.model.is_empty()
        || boundary.context_schema_key.is_empty()
        || boundary.dialect.is_empty()
        || boundary.adapter_revision.is_empty()
        || boundary.agent_key.as_ref().is_some_and(String::is_empty)
        || boundary
            .model_deployment_key
            .as_ref()
            .is_some_and(String::is_empty)
        || boundary.kv_namespace.as_ref().is_some_and(String::is_empty)
    {
        bail!("Session boundary context keys must be non-empty");
    }
    Ok(())
}

fn parsed_timestamp(value: Option<&str>) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(value?, &Rfc3339).ok()
}

fn request_order(
    previous: &actrail_kv_artifacts::SessionRequestReference,
    current: &actrail_kv_artifacts::SessionRequestReference,
) -> std::cmp::Ordering {
    parsed_timestamp(previous.captured_at.as_deref())
        .cmp(&parsed_timestamp(current.captured_at.as_deref()))
        .then_with(|| previous.input_line.cmp(&current.input_line))
}
