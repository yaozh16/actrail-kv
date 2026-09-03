//! 本文件按显式 Session 与时间顺序构建相邻请求 transition。

use std::collections::{BTreeMap, BTreeSet};

use actrail_kv_artifacts::{
    SessionRequestReference, SessionTimeline, SessionTransition, SessionTransitionOutcome,
};
use sha2::{Digest, Sha256};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::model::projection::CacheSequence;

use super::{model, prefix, SessionObservation};

pub fn build(
    observations: Vec<SessionObservation>,
    sequences: &[CacheSequence],
) -> Vec<SessionTimeline> {
    let mut grouped = BTreeMap::<String, Vec<SessionObservation>>::new();
    for observation in observations {
        grouped
            .entry(observation.session_id.clone())
            .or_default()
            .push(observation);
    }
    grouped
        .into_iter()
        .map(|(session_id, mut observations)| {
            let reliably_ordered = observations.iter().all(|item| parsed_at(item).is_some());
            if reliably_ordered {
                observations.sort_by(|left, right| {
                    parsed_at(left)
                        .expect("validated timestamp")
                        .cmp(&parsed_at(right).expect("validated timestamp"))
                        .then_with(|| left.input_line.cmp(&right.input_line))
                });
            } else {
                // Invalid timestamps have no defensible place on the time axis. Input order is
                // retained only for deterministic display; every transition remains a boundary.
                observations.sort_by_key(|item| item.input_line);
            }
            timeline(session_id, observations, sequences, reliably_ordered)
        })
        .collect()
}

fn timeline(
    session_id: String,
    observations: Vec<SessionObservation>,
    sequences: &[CacheSequence],
    reliably_ordered: bool,
) -> SessionTimeline {
    let mut timestamp_counts = BTreeMap::<OffsetDateTime, usize>::new();
    for observation in &observations {
        if let Some(timestamp) = parsed_at(observation) {
            *timestamp_counts.entry(timestamp).or_default() += 1;
        }
    }
    let ambiguous_times: BTreeSet<_> = timestamp_counts
        .into_iter()
        .filter_map(|(timestamp, count)| (count > 1).then_some(timestamp))
        .collect();
    let requests = observations
        .iter()
        .map(|observation| SessionRequestReference {
            request_id: observation.request_id.clone(),
            captured_at: valid_captured_at(observation),
            input_line: observation.input_line,
            source: observation.source.clone(),
        })
        .collect();
    let transitions = observations
        .windows(2)
        .map(|pair| {
            transition(
                &session_id,
                &pair[0],
                &pair[1],
                &ambiguous_times,
                reliably_ordered,
                sequences,
            )
        })
        .collect();
    SessionTimeline {
        session_id,
        requests,
        transitions,
    }
}

fn transition(
    session_id: &str,
    previous: &SessionObservation,
    current: &SessionObservation,
    ambiguous_times: &BTreeSet<OffsetDateTime>,
    reliably_ordered: bool,
    sequences: &[CacheSequence],
) -> SessionTransition {
    let id = transition_id(session_id, previous, current);
    let (outcome, boundary) = if !reliably_ordered {
        (
            SessionTransitionOutcome::UnanalyzableBoundary {
                reason:
                    "session contains a missing or invalid captured_at; chronology is unavailable"
                        .to_owned(),
            },
            None,
        )
    } else {
        let left_time = parsed_at(previous).expect("reliably ordered timestamp");
        let right_time = parsed_at(current).expect("reliably ordered timestamp");
        if ambiguous_times.contains(&left_time) || ambiguous_times.contains(&right_time) {
            (
            SessionTransitionOutcome::AmbiguousOrder {
                reason: "requests share a captured_at value; input_line only provides deterministic order"
                    .to_owned(),
            },
            None,
            )
        } else {
            match (
                previous
                    .sequence_index
                    .and_then(|index| sequences.get(index)),
                current
                    .sequence_index
                    .and_then(|index| sequences.get(index)),
            ) {
                (Some(left), Some(right)) if !model::same_boundary(&left.domain, &right.domain) => {
                    (
                        SessionTransitionOutcome::IncomparableBoundary {
                            reason: "known comparison boundary changed".to_owned(),
                        },
                        None,
                    )
                }
                (Some(left), Some(right)) => (
                    prefix::compare(left, right),
                    Some(model::boundary(&left.domain)),
                ),
                _ => (
                    SessionTransitionOutcome::UnanalyzableBoundary {
                        reason: "one or both requests could not be projected".to_owned(),
                    },
                    None,
                ),
            }
        }
    };
    SessionTransition {
        id,
        previous_request_id: previous.request_id.clone(),
        current_request_id: current.request_id.clone(),
        previous_captured_at: valid_captured_at(previous),
        current_captured_at: valid_captured_at(current),
        boundary,
        outcome,
    }
}

fn valid_captured_at(observation: &SessionObservation) -> Option<String> {
    parsed_at(observation).map(|_| {
        observation
            .captured_at
            .clone()
            .expect("parsed timestamp exists")
    })
}

fn parsed_at(observation: &SessionObservation) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(observation.captured_at.as_deref()?, &Rfc3339).ok()
}

fn transition_id(
    session_id: &str,
    previous: &SessionObservation,
    current: &SessionObservation,
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"session-transition/v1\0");
    digest.update(session_id.as_bytes());
    digest.update(b"\0");
    digest.update(previous.request_id.as_bytes());
    digest.update(b"\0");
    digest.update(current.request_id.as_bytes());
    format!("transition-{}", hex::encode(digest.finalize()))
}
