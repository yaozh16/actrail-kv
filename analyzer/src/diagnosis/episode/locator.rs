//! 本文件将扫描器定位的 P1/X/P2 边界转换为直接缺陷或条件性局部候选。

use std::collections::BTreeMap;

use actrail_kv_artifacts::{ComparisonGroup, DefectFact, DefectFactKind};

use crate::diagnosis::{facts, identity};
use crate::discovery::template::{
    CoordinateBindingState, MemberCoordinateMap, RequestTemplate, TemplateCoordinate,
};
use crate::model::projection::{ComparisonDomain, ContextCollectionKind};

use super::scanner::{scan, EpisodeBounds};
use super::variant::{build_variant, episode_variant, member, recovered_evidence, LocatedVariant};
use super::{DefectCandidate, DiagnosisOptions, EpisodeKind};

pub fn diagnose_template(
    template: &RequestTemplate,
    options: &DiagnosisOptions,
) -> Vec<DefectCandidate> {
    if template.symbolic_members.len() < options.min_stable_support
        || template.coordinates.is_empty()
    {
        return vec![];
    }
    let maps: BTreeMap<_, _> = template
        .member_maps
        .iter()
        .map(|map| (map.member_request_id.as_str(), map))
        .collect();
    scan(template, options, |bounds, episode_index| {
        candidate(template, &maps, bounds, episode_index, options)
    })
}

fn candidate(
    template: &RequestTemplate,
    maps: &BTreeMap<&str, &MemberCoordinateMap>,
    bounds: &EpisodeBounds,
    episode_index: usize,
    options: &DiagnosisOptions,
) -> Option<DefectCandidate> {
    let local_p1_start = bounds.local_p1_start;
    let x_start = bounds.x_start;
    let recovery = &bounds.recovery;
    let mut located = Vec::new();
    for member_id in recovery.supporter_bytes.keys() {
        let Some(map) = maps.get(member_id.as_str()) else {
            continue;
        };
        let Some(member) = member(template, member_id) else {
            continue;
        };
        located.push(build_variant(
            template,
            member,
            map,
            x_start,
            recovery.start,
        ));
    }
    located.sort_by(|left, right| left.member_id.cmp(&right.member_id));
    if located.len() < options.min_stable_support {
        return None;
    }

    let mut histogram: BTreeMap<String, Vec<&LocatedVariant>> = BTreeMap::new();
    for variant in &located {
        histogram
            .entry(variant.fingerprint.clone())
            .or_default()
            .push(variant);
    }
    if histogram.len() < 2 {
        return None;
    }
    let largest = histogram
        .iter()
        .max_by(|left, right| {
            left.1
                .len()
                .cmp(&right.1.len())
                .then_with(|| right.0.cmp(left.0))
        })
        .map(|(fingerprint, _)| fingerprint.clone())
        .expect("nonempty histogram");
    let affected_count = located.len() - histogram[&largest].len();
    if affected_count == 0 {
        return None;
    }
    let representative = located
        .iter()
        .filter(|variant| variant.fingerprint != largest)
        .min_by(|left, right| {
            left.actual_prefix_bytes
                .cmp(&right.actual_prefix_bytes)
                .then_with(|| left.member_id.cmp(&right.member_id))
        })
        .expect("an affected variant exists");
    let local_prefix_bytes = if episode_index == 1 {
        representative.actual_prefix_bytes
    } else {
        maps.get(representative.member_id.as_str())
            .map(|map| observable_bytes(template, map, local_p1_start, x_start))
            .unwrap_or(0)
    };
    let blocked = located
        .iter()
        .filter(|variant| variant.fingerprint != largest)
        .filter_map(|variant| recovery.supporter_bytes.get(&variant.member_id))
        .copied()
        .min()
        .unwrap_or(0);
    if blocked < options.min_blocked_bytes {
        return None;
    }

    let fact_analysis = facts::analyze(&located, options.fixed_variant_max);
    let logical_shape = logical_shape(template, x_start, recovery.start);
    let logical_id = identity::defect_id(
        &template.domain,
        &logical_shape,
        &template.coordinates[x_start].id.0,
        recovery,
    );
    let id = if episode_index == 1 {
        logical_id
    } else {
        identity::conditional_site_id(&template.id, &logical_id)
    };
    let mut facts = fact_analysis.facts;
    facts.push(DefectFact {
        kind: DefectFactKind::Scope,
        detail: Some(logical_shape.clone()),
    });
    facts.sort_by(|left, right| format!("{:?}", left.kind).cmp(&format!("{:?}", right.kind)));
    let variants = histogram
        .into_iter()
        .map(|(fingerprint, members)| episode_variant(fingerprint, members))
        .collect();
    let recovered_stable = recovered_evidence(template, recovery, &representative.member_id);
    let confidence = (template.cohesion
        * template.projection_reliability
        * (located.len() as f64 / template.symbolic_members.len() as f64)
        * fact_analysis.certainty)
        .clamp(0.0, 1.0);
    Some(DefectCandidate {
        id,
        comparison_group: comparison_group(&template.domain),
        template_id: template.id.clone(),
        episode_index,
        kind: if episode_index == 1 {
            EpisodeKind::Direct
        } else {
            EpisodeKind::Conditional
        },
        pattern: fact_analysis.pattern,
        variants,
        facts,
        recovered_stable,
        representative_member_id: representative.member_id.clone(),
        actual_prefix_bytes: representative.actual_prefix_bytes,
        local_prefix_bytes,
        potential_prefix_bytes: representative.actual_prefix_bytes + blocked,
        blocked_stable_bytes: blocked,
        comparable_count: located.len(),
        affected_count,
        confidence,
        insights: fact_analysis.insights,
    })
}

fn observable_bytes(
    template: &RequestTemplate,
    map: &MemberCoordinateMap,
    start: usize,
    end: usize,
) -> usize {
    let binding_bytes: usize = map.bindings[start..end]
        .iter()
        .filter_map(|binding| match &binding.state {
            CoordinateBindingState::Present(fragment) => Some(fragment.observable_bytes),
            CoordinateBindingState::Gap => None,
        })
        .sum();
    let indexes: BTreeMap<_, _> = template
        .coordinates
        .iter()
        .enumerate()
        .map(|(index, coordinate)| (&coordinate.id, index))
        .collect();
    let unmatched_bytes: usize = map
        .unmatched_runs
        .iter()
        .filter(|run| {
            run.left_coordinate_id
                .as_ref()
                .and_then(|id| indexes.get(id).copied())
                .is_some_and(|index| index >= start)
                && run
                    .right_coordinate_id
                    .as_ref()
                    .and_then(|id| indexes.get(id).copied())
                    .is_some_and(|index| index < end)
        })
        .flat_map(|run| &run.fragments)
        .map(|fragment| fragment.observable_bytes)
        .sum();
    binding_bytes + unmatched_bytes
}

fn collection_name(collection: &ContextCollectionKind) -> &'static str {
    match collection {
        ContextCollectionKind::Request => "request",
        ContextCollectionKind::Tools => "tools",
        ContextCollectionKind::Messages => "messages",
        ContextCollectionKind::ContentBlocks => "content_blocks",
    }
}

fn logical_shape(template: &RequestTemplate, x_start: usize, p2_start: usize) -> String {
    template.coordinates[x_start..p2_start.max(x_start + 1).min(template.coordinates.len())]
        .iter()
        .map(coordinate_shape)
        .collect::<Vec<_>>()
        .join("/")
}

pub(super) fn coordinate_shape(coordinate: &TemplateCoordinate) -> String {
    format!(
        "{}:{:?}:{}:{}:{}",
        collection_name(&coordinate.logical_unit.collection),
        coordinate.logical_unit.unit_kind,
        coordinate.logical_unit.alignment_key,
        coordinate.logical_unit.role.as_deref().unwrap_or("-"),
        coordinate
            .logical_unit
            .stable_identity
            .as_deref()
            .unwrap_or("-")
    )
}

fn comparison_group(domain: &ComparisonDomain) -> ComparisonGroup {
    ComparisonGroup {
        time_window_key: domain.time_window_key.to_string(),
        endpoint_key: domain.endpoint_key.clone(),
        model: domain.model.clone(),
        context_schema_key: domain.context_schema_key.clone(),
        agent_key: domain.agent_key.clone(),
        model_deployment_key: domain.model_deployment_key.clone(),
        kv_namespace: domain.kv_namespace.clone(),
    }
}
