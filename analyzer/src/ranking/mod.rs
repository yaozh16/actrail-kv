//! 本模块按稳定根因指纹聚合成员影响并确定性生成 Top K。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::diagnosis::{Cause, DiagnosisObservation, SourceLocation};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub blocked_bytes: usize,
    pub affected: usize,
    pub confidence: f64,
    pub score: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RankedFinding {
    pub id: String,
    pub fingerprint: String,
    pub template_id: String,
    pub representative_member_id: String,
    pub cause: Cause,
    pub normalized_position: String,
    pub stable_anchor: String,
    pub actual_prefix_bytes: usize,
    pub potential_prefix_bytes: usize,
    pub blocked_stable_bytes: usize,
    pub affected: usize,
    pub support_count: usize,
    pub confidence: f64,
    pub score: ScoreBreakdown,
    pub source: SourceLocation,
    pub evidence: String,
    pub counterfactual: String,
    pub recommendation: String,
}

pub fn rank_findings(observations: &[DiagnosisObservation], top_k: usize) -> Vec<RankedFinding> {
    let mut groups: BTreeMap<String, Vec<&DiagnosisObservation>> = BTreeMap::new();
    for observation in observations {
        groups
            .entry(fingerprint(observation))
            .or_default()
            .push(observation);
    }
    let mut findings: Vec<_> = groups
        .into_iter()
        .filter_map(|(fingerprint, group)| aggregate(fingerprint, group))
        .collect();
    suppress_dominated(&mut findings);
    findings.sort_by(|left, right| {
        right
            .score
            .score
            .total_cmp(&left.score.score)
            .then_with(|| right.affected.cmp(&left.affected))
            .then_with(|| right.blocked_stable_bytes.cmp(&left.blocked_stable_bytes))
            .then_with(|| left.fingerprint.cmp(&right.fingerprint))
    });
    findings.truncate(top_k);
    findings
}

fn aggregate(fingerprint: String, mut group: Vec<&DiagnosisObservation>) -> Option<RankedFinding> {
    group.sort_by_key(|observation| &observation.member_id);
    let representative = group.iter().copied().min_by(|left, right| {
        left.blocked_stable_bytes
            .cmp(&right.blocked_stable_bytes)
            .then_with(|| left.actual_prefix_bytes.cmp(&right.actual_prefix_bytes))
            .then_with(|| {
                left.potential_prefix_bytes
                    .cmp(&right.potential_prefix_bytes)
            })
            .then_with(|| left.member_id.cmp(&right.member_id))
    })?;
    let support_count = representative.template_member_count;
    let mut variants = BTreeMap::<&str, usize>::new();
    for observation in &group {
        *variants
            .entry(&observation.variant_fingerprint)
            .or_default() += 1;
    }
    let baseline = support_count.saturating_sub(group.len());
    let largest_variant = variants.values().copied().max().unwrap_or(0).max(baseline);
    let affected = support_count.saturating_sub(largest_variant);
    if affected == 0 {
        return None;
    }
    let blocked = group
        .iter()
        .map(|observation| observation.blocked_stable_bytes)
        .min()?;
    let confidence = group
        .iter()
        .map(|observation| observation.confidence)
        .fold(1.0_f64, f64::min);
    let score = blocked as f64 * affected as f64 * confidence;
    Some(RankedFinding {
        id: format!("finding-{}", &fingerprint[..16]),
        fingerprint: fingerprint.clone(),
        template_id: representative.template_id.clone(),
        representative_member_id: representative.member_id.clone(),
        cause: representative.cause.clone(),
        normalized_position: representative.normalized_position.clone(),
        stable_anchor: representative.stable_anchor.clone(),
        actual_prefix_bytes: representative.actual_prefix_bytes,
        potential_prefix_bytes: representative.potential_prefix_bytes,
        blocked_stable_bytes: blocked,
        affected,
        support_count,
        confidence,
        score: ScoreBreakdown {
            blocked_bytes: blocked,
            affected,
            confidence,
            score,
        },
        source: representative.source.clone(),
        evidence: representative.evidence.clone(),
        counterfactual: representative.counterfactual.clone(),
        recommendation: representative.recommendation.clone(),
    })
}

fn fingerprint(observation: &DiagnosisObservation) -> String {
    let material = format!(
        "{}\0{:?}\0{}\0{}\0{}",
        observation.template_id,
        observation.cause,
        observation.normalized_position,
        observation.stable_anchor,
        observation.recommendation
    );
    hex::encode(Sha256::digest(material.as_bytes()))
}

fn suppress_dominated(findings: &mut Vec<RankedFinding>) {
    findings.sort_by(|left, right| {
        right
            .blocked_stable_bytes
            .cmp(&left.blocked_stable_bytes)
            .then_with(|| left.fingerprint.cmp(&right.fingerprint))
    });
    let mut kept: Vec<RankedFinding> = Vec::new();
    for finding in findings.drain(..) {
        let dominated = kept.iter().any(|existing| {
            existing.template_id == finding.template_id
                && existing.normalized_position == finding.normalized_position
                && existing.recommendation == finding.recommendation
                && existing.blocked_stable_bytes >= finding.blocked_stable_bytes
        });
        if !dominated {
            kept.push(finding);
        }
    }
    *findings = kept;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(
        member: &str,
        variant: &str,
        blocked: usize,
        confidence: f64,
    ) -> DiagnosisObservation {
        DiagnosisObservation {
            template_id: "template".into(),
            template_member_count: 5,
            member_id: member.into(),
            cause: Cause::EarlyVolatileContent,
            normalized_position: "unit:0".into(),
            stable_anchor: "stable".into(),
            actual_prefix_bytes: 10,
            potential_prefix_bytes: 10 + blocked,
            blocked_stable_bytes: blocked,
            confidence,
            source: SourceLocation {
                json_path: "$[0]".into(),
                utf8_range: Some((0, 0)),
            },
            evidence: "evidence".into(),
            counterfactual: "counterfactual".into(),
            recommendation: "recommendation".into(),
            variant_fingerprint: variant.into(),
        }
    }

    #[test]
    fn affected_is_population_minus_largest_variant_not_pair_count() {
        let observations = vec![
            observation("b", "variant-x", 100, 0.9),
            observation("c", "variant-x", 120, 0.8),
            observation("d", "variant-y", 110, 0.95),
        ];
        let finding = rank_findings(&observations, 10).pop().unwrap();
        assert_eq!(finding.support_count, 5);
        assert_eq!(finding.affected, 3); // frequencies: baseline=2, x=2, y=1
        assert_eq!(finding.blocked_stable_bytes, 100);
        assert_eq!(finding.score.score, 240.0);
    }

    #[test]
    fn ordering_and_top_k_are_input_permutation_invariant() {
        let mut first = observation("a", "x", 100, 1.0);
        let mut second = observation("b", "y", 200, 1.0);
        second.normalized_position = "unit:1".into();
        let forward = rank_findings(&[first.clone(), second.clone()], 1);
        let reverse = rank_findings(&[second, first.clone()], 1);
        assert_eq!(forward, reverse);
        assert_eq!(forward[0].normalized_position, "unit:1");

        first.member_id = "z".into();
        assert_eq!(rank_findings(&[first], 0), Vec::<RankedFinding>::new());
    }

    #[test]
    fn same_root_is_aggregated_once() {
        let findings = rank_findings(
            &[
                observation("b", "x", 100, 1.0),
                observation("c", "y", 100, 1.0),
            ],
            10,
        );
        assert_eq!(findings.len(), 1);
        assert!(findings[0].id.starts_with("finding-"));
    }

    #[test]
    fn metrics_come_from_one_conservative_observation() {
        let mut low_blocked = observation("b", "x", 50, 1.0);
        low_blocked.actual_prefix_bytes = 100;
        low_blocked.potential_prefix_bytes = 150;
        let mut low_actual = observation("c", "y", 100, 1.0);
        low_actual.actual_prefix_bytes = 10;
        low_actual.potential_prefix_bytes = 110;
        let finding = rank_findings(&[low_actual, low_blocked], 10).pop().unwrap();
        assert_eq!(finding.actual_prefix_bytes, 100);
        assert_eq!(finding.blocked_stable_bytes, 50);
        assert_eq!(finding.potential_prefix_bytes, 150);
        assert_eq!(
            finding.potential_prefix_bytes,
            finding.actual_prefix_bytes + finding.blocked_stable_bytes
        );
    }

    #[test]
    fn equal_dominance_ties_choose_lexicographically_stable_fingerprint() {
        let mut left = observation("b", "x", 100, 1.0);
        left.stable_anchor = "left".into();
        let mut right = observation("c", "y", 100, 1.0);
        right.stable_anchor = "right".into();
        let forward = rank_findings(&[left.clone(), right.clone()], 10);
        let reverse = rank_findings(&[right, left], 10);
        assert_eq!(forward, reverse);
        assert_eq!(forward.len(), 1);
    }
}
