//! 本模块只对已唯一化的 region 候选做保守评分、确定性全排序和 Top K 前缀选择。

use std::collections::{BTreeMap, BTreeSet};

use actrail_kv_artifacts::{
    ConditionalLocalSite, ContextDefect, MismatchRegion, MismatchVariant, ScoreBreakdown,
};

use crate::diagnosis::{DefectCandidate, EpisodeKind};

#[derive(Clone, Debug, PartialEq)]
pub struct RankedDefects {
    pub defects: Vec<ContextDefect>,
    pub conditional_local_sites: Vec<ConditionalLocalSite>,
    pub top_k: Vec<String>,
}

pub fn rank_defects(candidates: Vec<DefectCandidate>, top_k: usize) -> RankedDefects {
    let (direct, conditional): (Vec<_>, Vec<_>) = candidates
        .into_iter()
        .partition(|candidate| candidate.kind == EpisodeKind::Direct);
    let mut unique = BTreeMap::<String, DefectCandidate>::new();
    for candidate in direct {
        unique
            .entry(candidate.id.clone())
            .and_modify(|existing| {
                if candidate_order(&candidate, existing).is_lt() {
                    *existing = candidate.clone();
                }
            })
            .or_insert(candidate);
    }
    let mut defects: Vec<_> = unique.into_values().map(into_artifact).collect();
    defects.sort_by(|left, right| {
        right
            .score
            .score
            .total_cmp(&left.score.score)
            .then_with(|| right.affected_count.cmp(&left.affected_count))
            .then_with(|| right.blocked_stable_bytes.cmp(&left.blocked_stable_bytes))
            .then_with(|| left.id.cmp(&right.id))
    });
    let top_k = defects
        .iter()
        .take(top_k)
        .map(|defect| defect.id.clone())
        .collect();
    let direct_templates: BTreeSet<_> = defects
        .iter()
        .map(|defect| defect.template_id.as_str())
        .collect();
    let direct_ids: BTreeSet<_> = defects.iter().map(|defect| defect.id.as_str()).collect();
    let mut conditional_local_sites: Vec<_> = unique_conditional_candidates(
        conditional
            .into_iter()
            .filter(|candidate| direct_templates.contains(candidate.template_id.as_str()))
            .filter(|candidate| !direct_ids.contains(candidate.id.as_str()))
            .collect(),
    )
    .into_values()
    .map(into_conditional_artifact)
    .collect();
    conditional_local_sites.sort_by(|left, right| {
        left.template_id
            .cmp(&right.template_id)
            .then_with(|| left.episode_index.cmp(&right.episode_index))
            .then_with(|| left.id.cmp(&right.id))
    });
    RankedDefects {
        defects,
        conditional_local_sites,
        top_k,
    }
}

fn unique_conditional_candidates(
    candidates: Vec<DefectCandidate>,
) -> BTreeMap<(String, String), DefectCandidate> {
    let mut unique = BTreeMap::new();
    for candidate in candidates {
        let key = (candidate.template_id.clone(), candidate.id.clone());
        unique
            .entry(key)
            .and_modify(|existing| {
                if candidate_order(&candidate, existing).is_lt() {
                    *existing = candidate.clone();
                }
            })
            .or_insert(candidate);
    }
    unique
}

fn into_artifact(candidate: DefectCandidate) -> ContextDefect {
    debug_assert_eq!(
        candidate.potential_prefix_bytes,
        candidate.actual_prefix_bytes + candidate.blocked_stable_bytes
    );
    let score = candidate.blocked_stable_bytes as f64
        * candidate.affected_count as f64
        * candidate.confidence;
    ContextDefect {
        id: candidate.id,
        comparison_group: candidate.comparison_group,
        template_id: candidate.template_id,
        mismatch: MismatchRegion {
            pattern: candidate.pattern,
            variants: candidate
                .variants
                .into_iter()
                .map(|variant| MismatchVariant {
                    fingerprint: variant.fingerprint,
                    member_request_ids: variant.member_request_ids,
                    representative: variant.representative,
                })
                .collect(),
            facts: candidate.facts,
        },
        recovered_stable: candidate.recovered_stable,
        actual_prefix_bytes: candidate.actual_prefix_bytes,
        potential_prefix_bytes: candidate.potential_prefix_bytes,
        blocked_stable_bytes: candidate.blocked_stable_bytes,
        comparable_count: candidate.comparable_count,
        affected_count: candidate.affected_count,
        confidence: candidate.confidence,
        score: ScoreBreakdown {
            blocked_stable_bytes: candidate.blocked_stable_bytes,
            affected_count: candidate.affected_count,
            confidence: candidate.confidence,
            score,
        },
        insights: candidate.insights,
    }
}

fn into_conditional_artifact(candidate: DefectCandidate) -> ConditionalLocalSite {
    ConditionalLocalSite {
        id: candidate.id,
        comparison_group: candidate.comparison_group,
        template_id: candidate.template_id,
        episode_index: candidate.episode_index,
        mismatch: mismatch_region(candidate.pattern, candidate.variants, candidate.facts),
        recovered_stable: candidate.recovered_stable,
        local_prefix_bytes: candidate.local_prefix_bytes,
        blocked_stable_bytes: candidate.blocked_stable_bytes,
        comparable_count: candidate.comparable_count,
        affected_count: candidate.affected_count,
        confidence: candidate.confidence,
        insights: candidate.insights,
    }
}

fn mismatch_region(
    pattern: actrail_kv_artifacts::MismatchPattern,
    variants: Vec<crate::diagnosis::EpisodeVariant>,
    facts: Vec<actrail_kv_artifacts::DefectFact>,
) -> MismatchRegion {
    MismatchRegion {
        pattern,
        variants: variants
            .into_iter()
            .map(|variant| MismatchVariant {
                fingerprint: variant.fingerprint,
                member_request_ids: variant.member_request_ids,
                representative: variant.representative,
            })
            .collect(),
        facts,
    }
}

fn candidate_order(left: &DefectCandidate, right: &DefectCandidate) -> std::cmp::Ordering {
    left.blocked_stable_bytes
        .cmp(&right.blocked_stable_bytes)
        .then_with(|| left.affected_count.cmp(&right.affected_count))
        .then_with(|| left.confidence.total_cmp(&right.confidence))
        .then_with(|| {
            left.representative_member_id
                .cmp(&right.representative_member_id)
        })
}

#[cfg(test)]
mod tests;
