//! 本文件将重复出现的 Session 历史变化聚合为稳定逻辑位点。

use std::collections::{BTreeMap, BTreeSet};

use actrail_kv_artifacts::{
    OptimizationInsight, SessionBoundaryContext, SessionHistorySite, SessionHistorySiteKind,
    SessionTimeline, SessionTransitionOutcome,
};
use sha2::{Digest, Sha256};

#[derive(Default)]
struct SiteAccumulator {
    transitions: Vec<String>,
    sessions: BTreeSet<String>,
    invalidated: Vec<usize>,
}

pub fn history_sites(timelines: &[SessionTimeline]) -> Vec<SessionHistorySite> {
    let mut sites =
        BTreeMap::<(SessionBoundaryContext, SessionHistorySiteKind, String), SiteAccumulator>::new(
        );
    for timeline in timelines {
        for transition in &timeline.transitions {
            let (kind, metrics, divergence) = match &transition.outcome {
                SessionTransitionOutcome::HistoryChanged {
                    metrics,
                    divergence,
                } => (SessionHistorySiteKind::HistoryChanged, metrics, divergence),
                SessionTransitionOutcome::PrefixTruncated {
                    metrics,
                    divergence,
                } => (SessionHistorySiteKind::PrefixTruncated, metrics, divergence),
                _ => continue,
            };
            let boundary = transition
                .boundary
                .clone()
                .expect("comparable history transition has a boundary");
            let site = sites
                .entry((boundary, kind, divergence.logical_position.clone()))
                .or_default();
            site.transitions.push(transition.id.clone());
            site.sessions.insert(timeline.session_id.clone());
            site.invalidated
                .push(metrics.invalidated_previous_suffix_bytes);
        }
    }
    sites
        .into_iter()
        .map(|((boundary, kind, logical_position), mut site)| {
            site.transitions.sort();
            site.invalidated.sort_unstable();
            let occurrence_count = site.transitions.len();
            SessionHistorySite {
                id: site_id(&boundary, &kind, &logical_position),
                kind: kind.clone(),
                boundary,
                logical_position,
                transition_ids: site.transitions,
                occurrence_count,
                affected_session_count: site.sessions.len(),
                invalidated_previous_suffix_bytes_total: site.invalidated.iter().sum(),
                invalidated_previous_suffix_bytes_min: site.invalidated[0],
                invalidated_previous_suffix_bytes_max: *site
                    .invalidated
                    .last()
                    .expect("history site contains an observation"),
                insight: insight(occurrence_count, &kind),
            }
        })
        .collect()
}

fn insight(occurrence_count: usize, kind: &SessionHistorySiteKind) -> Option<OptimizationInsight> {
    (occurrence_count >= 2).then(|| match kind {
        SessionHistorySiteKind::HistoryChanged => OptimizationInsight {
            summary: "保持此位置之前的既有 Session 历史不变".to_owned(),
            detail: Some("将新状态追加到尾部，避免重复改写已经出现的上下文".to_owned()),
        },
        SessionHistorySiteKind::PrefixTruncated => OptimizationInsight {
            summary: "减少对既有 Session 历史的前缀截断".to_owned(),
            detail: Some("在明确边界集中执行历史压缩，避免每轮从头截断".to_owned()),
        },
    })
}

fn site_id(
    boundary: &SessionBoundaryContext,
    kind: &SessionHistorySiteKind,
    logical_position: &str,
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"session-history-site/v1\0");
    digest_field(&mut digest, &boundary.endpoint_key);
    digest_field(&mut digest, &boundary.model);
    digest_field(&mut digest, &boundary.context_schema_key);
    digest_optional(&mut digest, boundary.agent_key.as_deref());
    digest_optional(&mut digest, boundary.model_deployment_key.as_deref());
    digest_optional(&mut digest, boundary.kv_namespace.as_deref());
    digest_field(&mut digest, &boundary.dialect);
    digest_field(&mut digest, &boundary.adapter_revision);
    digest_field(
        &mut digest,
        match kind {
            SessionHistorySiteKind::HistoryChanged => "history_changed",
            SessionHistorySiteKind::PrefixTruncated => "prefix_truncated",
        },
    );
    digest.update(logical_position.as_bytes());
    format!("history-site-{}", hex::encode(digest.finalize()))
}

fn digest_optional(digest: &mut Sha256, value: Option<&str>) {
    match value {
        Some(value) => {
            digest.update(b"some\0");
            digest_field(digest, value);
        }
        None => digest.update(b"none\0"),
    }
}

fn digest_field(digest: &mut Sha256, value: &str) {
    digest.update(value.len().to_le_bytes());
    digest.update(value.as_bytes());
}
