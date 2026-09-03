//! 本文件以先验证后提交的单链状态机扫描有序、非重叠 X/P2 episode。

use std::collections::{BTreeMap, BTreeSet};

use crate::diagnosis::identity;
use crate::discovery::template::{
    CoordinateBindingState, CoordinateKind, MemberCoordinateMap, MemberFragmentRef,
    RequestTemplate, TemplateCoordinateId,
};

use super::DiagnosisOptions;

#[derive(Clone, Debug)]
pub(in crate::diagnosis) struct Recovery {
    pub start: usize,
    pub end: usize,
    pub supporter_bytes: BTreeMap<String, usize>,
    pub anchor_chain_digest: String,
    pub local_shape: String,
}

#[derive(Clone, Debug)]
pub(super) struct EpisodeBounds {
    pub local_p1_start: usize,
    pub x_start: usize,
    pub recovery: Recovery,
}

pub(super) fn scan<T>(
    template: &RequestTemplate,
    options: &DiagnosisOptions,
    mut validate: impl FnMut(&EpisodeBounds, usize) -> Option<T>,
) -> Vec<T> {
    if options.max_episodes_per_template == 0 {
        return Vec::new();
    }
    let maps: BTreeMap<_, _> = template
        .member_maps
        .iter()
        .map(|map| (map.member_request_id.as_str(), map))
        .collect();
    let mut active: BTreeSet<String> = maps.keys().map(|member| (*member).to_owned()).collect();
    let mut cursor = 0;
    let mut local_p1_start = 0;
    let mut accepted = Vec::new();

    while accepted.len() < options.max_episodes_per_template
        && active.len() >= options.min_stable_support
        && cursor < template.coordinates.len()
    {
        let Some((x_start, recovery_search)) = next_mismatch(template, &maps, &active, cursor)
        else {
            break;
        };
        let Some(recovery) = find_recovery(template, &maps, &active, recovery_search, options)
        else {
            break;
        };
        let bounds = EpisodeBounds {
            local_p1_start,
            x_start,
            recovery,
        };

        // Rejected regions advance only the search cursor. They do not consume numbering or
        // narrow active supporters, so a later valid region can still become the direct defect.
        cursor = bounds.recovery.end;
        if let Some(value) = validate(&bounds, accepted.len() + 1) {
            local_p1_start = bounds.recovery.start;
            active = bounds.recovery.supporter_bytes.keys().cloned().collect();
            accepted.push(value);
        }
    }
    accepted
}

fn next_mismatch(
    template: &RequestTemplate,
    maps: &BTreeMap<&str, &MemberCoordinateMap>,
    active: &BTreeSet<String>,
    cursor: usize,
) -> Option<(usize, usize)> {
    let indexes = coordinate_indexes(template);
    for index in cursor..template.coordinates.len() {
        let unmatched: BTreeSet<_> = active
            .iter()
            .map(|member| unmatched_signature(maps[member.as_str()], index, &indexes))
            .collect();
        if unmatched.len() > 1 {
            return Some((index, index));
        }
        let bindings: BTreeSet<_> = active
            .iter()
            .map(|member| binding_signature(maps[member.as_str()], index))
            .collect();
        if bindings.len() > 1 {
            return Some((index, index + 1));
        }
    }
    None
}

fn find_recovery(
    template: &RequestTemplate,
    maps: &BTreeMap<&str, &MemberCoordinateMap>,
    active: &BTreeSet<String>,
    search: usize,
    options: &DiagnosisOptions,
) -> Option<Recovery> {
    for start in search..template.coordinates.len() {
        let Some(start_digest) = stable_digest(template, maps, active.iter(), start) else {
            continue;
        };
        let mut supporters: BTreeMap<String, usize> = active
            .iter()
            .filter_map(|member| {
                digest_fragment(maps[member.as_str()], start, &start_digest)
                    .map(|fragment| (member.clone(), fragment.observable_bytes))
            })
            .collect();
        let mut coordinate_digests = vec![start_digest];
        let mut anchor_end = None;
        let mut anchor_chain_digest = None;
        let mut end = start + 1;

        while end <= template.coordinates.len() {
            if end > start + 1 {
                let index = end - 1;
                let Some(digest) = stable_digest(template, maps, supporters.keys(), index) else {
                    break;
                };
                let coordinate = &template.coordinates[index];
                let previous = &template.coordinates[index - 1];
                let run_signatures: BTreeSet<_> = supporters
                    .keys()
                    .map(|member| {
                        unmatched_signature_for_right(maps[member.as_str()], &coordinate.id)
                    })
                    .collect();
                if run_signatures.len() > 1 {
                    break;
                }

                let before = supporters.clone();
                supporters.retain(|member, bytes| {
                    let map = maps[member.as_str()];
                    digest_fragment(map, index, &digest).is_some_and(|fragment| {
                        *bytes += fragment.observable_bytes
                            + stable_run_bytes(map, &previous.id, &coordinate.id);
                        true
                    })
                });
                if anchor_end.is_some() && supporters.len() != before.len() {
                    supporters = before;
                    break;
                }
                coordinate_digests.push(digest);
            }

            let min_bytes = supporters.values().copied().min().unwrap_or(0);
            if anchor_end.is_none()
                && supported(supporters.len(), template.symbolic_members.len(), options)
                && min_bytes >= options.min_blocked_bytes
                && min_bytes >= options.min_anchor_bytes
            {
                anchor_end = Some(end);
                anchor_chain_digest = Some(anchor_digest(
                    template,
                    maps,
                    supporters.keys().next().expect("qualified supporter"),
                    start,
                    end,
                    &coordinate_digests,
                ));
            }
            end += 1;
        }

        let Some(anchor_end) = anchor_end else {
            continue;
        };
        let recovery_end = if end > template.coordinates.len() {
            template.coordinates.len()
        } else {
            end - 1
        };
        let local_shape = template.coordinates[start..anchor_end]
            .iter()
            .map(super::locator::coordinate_shape)
            .collect::<Vec<_>>()
            .join("/");
        return Some(Recovery {
            start,
            end: recovery_end,
            supporter_bytes: supporters,
            anchor_chain_digest: anchor_chain_digest.expect("qualified anchor has identity"),
            local_shape,
        });
    }
    None
}

fn anchor_digest(
    template: &RequestTemplate,
    maps: &BTreeMap<&str, &MemberCoordinateMap>,
    supporter: &str,
    start: usize,
    end: usize,
    coordinate_digests: &[String],
) -> String {
    let mut material = Vec::new();
    for (offset, coordinate) in template.coordinates[start..end].iter().enumerate() {
        if offset > 0 {
            material
                .push(unmatched_signature_for_right(maps[supporter], &coordinate.id).into_bytes());
        }
        material.push(coordinate.id.0.as_bytes().to_vec());
        material.push(coordinate_digests[offset].as_bytes().to_vec());
    }
    identity::digest(material)
}

fn supported(count: usize, total: usize, options: &DiagnosisOptions) -> bool {
    count >= options.min_stable_support
        && count as f64 / total.max(1) as f64 + f64::EPSILON >= options.stable_support_rate
}

fn stable_digest<'a>(
    template: &RequestTemplate,
    maps: &BTreeMap<&str, &MemberCoordinateMap>,
    members: impl Iterator<Item = &'a String>,
    index: usize,
) -> Option<String> {
    if template.coordinates[index].kind == CoordinateKind::Stable {
        return Some(template.coordinates[index].content_digest.clone());
    }
    let signatures: BTreeSet<_> = members
        .map(|member| binding_signature(maps[member.as_str()], index))
        .collect();
    (signatures.len() == 1)
        .then(|| signatures.into_iter().next().expect("one signature"))
        .filter(|digest| digest != "<gap>")
}

fn binding_signature(map: &MemberCoordinateMap, index: usize) -> String {
    match &map.bindings[index].state {
        CoordinateBindingState::Present(fragment) => fragment.content_digest.clone(),
        CoordinateBindingState::Gap => "<gap>".into(),
    }
}

fn digest_fragment<'a>(
    map: &'a MemberCoordinateMap,
    index: usize,
    digest: &str,
) -> Option<&'a MemberFragmentRef> {
    match &map.bindings.get(index)?.state {
        CoordinateBindingState::Present(fragment) if fragment.content_digest == digest => {
            Some(fragment)
        }
        _ => None,
    }
}

fn stable_run_bytes(
    map: &MemberCoordinateMap,
    left: &TemplateCoordinateId,
    right: &TemplateCoordinateId,
) -> usize {
    map.unmatched_runs
        .iter()
        .filter(|run| {
            run.left_coordinate_id.as_ref() == Some(left)
                && run.right_coordinate_id.as_ref() == Some(right)
        })
        .flat_map(|run| &run.fragments)
        .map(|fragment| fragment.observable_bytes)
        .sum()
}

fn unmatched_signature(
    map: &MemberCoordinateMap,
    index: usize,
    indexes: &BTreeMap<&TemplateCoordinateId, usize>,
) -> String {
    identity::digest(
        map.unmatched_runs
            .iter()
            .filter(|run| boundary_index(run.right_coordinate_id.as_ref(), indexes) == Some(index))
            .flat_map(|run| {
                run.left_coordinate_id
                    .iter()
                    .map(|id| id.0.as_bytes())
                    .chain(
                        run.fragments
                            .iter()
                            .map(|fragment| fragment.content_digest.as_bytes()),
                    )
            }),
    )
}

fn unmatched_signature_for_right(
    map: &MemberCoordinateMap,
    right: &TemplateCoordinateId,
) -> String {
    identity::digest(
        map.unmatched_runs
            .iter()
            .filter(|run| run.right_coordinate_id.as_ref() == Some(right))
            .flat_map(|run| {
                run.left_coordinate_id
                    .iter()
                    .map(|id| id.0.as_bytes())
                    .chain(
                        run.fragments
                            .iter()
                            .map(|fragment| fragment.content_digest.as_bytes()),
                    )
            }),
    )
}

fn coordinate_indexes(template: &RequestTemplate) -> BTreeMap<&TemplateCoordinateId, usize> {
    template
        .coordinates
        .iter()
        .enumerate()
        .map(|(index, coordinate)| (&coordinate.id, index))
        .collect()
}

fn boundary_index(
    id: Option<&TemplateCoordinateId>,
    indexes: &BTreeMap<&TemplateCoordinateId, usize>,
) -> Option<usize> {
    id.and_then(|id| indexes.get(id).copied())
}
