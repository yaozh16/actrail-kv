//! 本模块提供 episode 测试 fixture，并验证基础与多位点扫描行为。

use std::ops::Range;

use actrail_kv_artifacts::{DefectFactKind, MismatchPattern};

use super::*;
use crate::discovery::template::{
    CoordinateBinding, CoordinateBindingState, CoordinateKind, LogicalUnitKey, MemberCoordinateMap,
    MemberFragmentRef, RequestTemplate, SymbolicMemberSequence, TemplateCoordinate,
    TemplateCoordinateId, UnmatchedRun,
};
use crate::model::projection::{
    CacheSequence, CacheUnit, CacheUnitKind, ComparisonDomain, ContextCollectionKind,
    HierarchyLocation, SourceLocation,
};

mod review;

fn sequence(id: &str, values: &[&str]) -> CacheSequence {
    CacheSequence {
        request_id: id.into(),
        domain: domain(),
        units: values
            .iter()
            .enumerate()
            .map(|(index, value)| CacheUnit {
                kind: CacheUnitKind::VisibleText,
                alignment_key: format!("message:system:content:{index}"),
                content: (*value).into(),
                source: source(index, value.len()),
                tool_identity: None,
                hierarchy: HierarchyLocation {
                    collection: ContextCollectionKind::Messages,
                    parent_json_path: "$.messages".into(),
                    element_index: Some(index),
                    content_block_index: None,
                },
            })
            .collect(),
        projection_reliability_millis: 1000,
    }
}

fn template(values: &[&str], p2: &str) -> RequestTemplate {
    let members: Vec<_> = values
        .iter()
        .enumerate()
        .map(|(index, value)| sequence(&format!("member-{index}"), &["prefix", value, p2]))
        .collect();
    let coordinates = vec![
        coordinate(
            "prefix",
            0,
            CoordinateKind::Stable,
            "prefix",
            6,
            values.len(),
        ),
        coordinate(
            "slot",
            1,
            CoordinateKind::Slot,
            "slot-medoid",
            11,
            values.len(),
        ),
        coordinate("p2", 2, CoordinateKind::Stable, p2, p2.len(), values.len()),
    ];
    let member_maps = members
        .iter()
        .zip(values)
        .map(|(member, value)| MemberCoordinateMap {
            member_request_id: member.request_id.clone(),
            bindings: vec![
                binding("prefix", 0, "prefix"),
                binding("slot", 1, value),
                binding("p2", 2, p2),
            ],
            unmatched_runs: vec![],
        })
        .collect();
    let symbolic_members = members
        .iter()
        .map(|member| SymbolicMemberSequence {
            member_request_id: member.request_id.clone(),
            atoms: vec![],
            observable_bytes: member.units.iter().map(CacheUnit::observable_bytes).sum(),
        })
        .collect();
    RequestTemplate {
        id: "template-instance".into(),
        domain: domain(),
        medoid_request_id: "member-0".into(),
        members,
        stable_spans: vec![],
        slots: vec![],
        coordinates,
        member_maps,
        symbolic_members,
        cohesion: 0.9,
        projection_reliability: 1.0,
    }
}

fn multi_episode_template(rows: &[Vec<&str>], kinds: &[CoordinateKind]) -> RequestTemplate {
    assert!(!rows.is_empty());
    assert!(rows.iter().all(|row| row.len() == kinds.len()));
    let members: Vec<_> = rows
        .iter()
        .enumerate()
        .map(|(index, row)| sequence(&format!("member-{index}"), row))
        .collect();
    let coordinates: Vec<_> = kinds
        .iter()
        .enumerate()
        .map(|(index, kind)| {
            coordinate(
                &format!("coordinate-{index}"),
                index,
                kind.clone(),
                rows[0][index],
                rows[0][index].len(),
                rows.len(),
            )
        })
        .collect();
    let member_maps = members
        .iter()
        .zip(rows)
        .map(|(member, row)| MemberCoordinateMap {
            member_request_id: member.request_id.clone(),
            bindings: row
                .iter()
                .enumerate()
                .map(|(index, content)| binding(&format!("coordinate-{index}"), index, content))
                .collect(),
            unmatched_runs: vec![],
        })
        .collect();
    let symbolic_members = members
        .iter()
        .map(|member| SymbolicMemberSequence {
            member_request_id: member.request_id.clone(),
            atoms: vec![],
            observable_bytes: member.units.iter().map(CacheUnit::observable_bytes).sum(),
        })
        .collect();
    RequestTemplate {
        id: "multi-template".into(),
        domain: domain(),
        medoid_request_id: "member-0".into(),
        members,
        stable_spans: vec![],
        slots: vec![],
        coordinates,
        member_maps,
        symbolic_members,
        cohesion: 0.9,
        projection_reliability: 1.0,
    }
}

#[test]
fn content_variants_form_one_episode_and_count_each_member_once() {
    let p2 = "stable recovery".repeat(8);
    let defect = diagnose_template(
        &template(&["north", "south", "east", "west"], &p2),
        &DiagnosisOptions::default(),
    )
    .pop()
    .unwrap();
    assert_eq!(defect.pattern, MismatchPattern::ValueMismatch);
    assert_eq!(defect.comparable_count, 4);
    assert_eq!(defect.affected_count, 3);
    assert_eq!(defect.variants.len(), 4);
    assert_eq!(defect.blocked_stable_bytes, p2.len());
    assert!(defect
        .facts
        .iter()
        .any(|fact| fact.kind == DefectFactKind::ContentVariation));
}

#[test]
fn no_qualified_p2_is_a_normal_dynamic_suffix() {
    let result = diagnose_template(
        &template(&["north", "south", "east", "west"], "short"),
        &DiagnosisOptions::default(),
    );
    assert!(result.is_empty());
}

#[test]
fn json_equivalence_is_a_fact_on_the_same_episode() {
    let p2 = "stable recovery".repeat(8);
    let defect = diagnose_template(
        &template(
            &[
                "{\"a\":1,\"b\":2}",
                "{ \"b\": 2, \"a\": 1 }",
                "{\n\"a\":1,\"b\":2}",
                "{\"b\":2,\"a\":1}",
            ],
            &p2,
        ),
        &DiagnosisOptions::default(),
    )
    .pop()
    .unwrap();
    assert!(defect
        .facts
        .iter()
        .any(|fact| fact.kind == DefectFactKind::StructuredDataEquivalent));
    assert_eq!(defect.variants.len(), 4);
}

#[test]
fn input_and_member_order_do_not_change_region_identity() {
    let p2 = "stable recovery".repeat(8);
    let original = template(&["north", "south", "east", "west"], &p2);
    let mut reversed = original.clone();
    reversed.members.reverse();
    reversed.member_maps.reverse();
    reversed.symbolic_members.reverse();
    let first = diagnose_template(&original, &DiagnosisOptions::default());
    let second = diagnose_template(&reversed, &DiagnosisOptions::default());
    assert_eq!(first, second);
}

#[test]
fn emits_direct_then_conditional_episode_with_maximal_non_overlapping_p2() {
    let stable_a = "first stable recovery ".repeat(4);
    let stable_b = "second stable recovery ".repeat(4);
    let rows = ["north", "south", "east", "west"]
        .into_iter()
        .map(|variant| {
            vec![
                "prefix",
                variant,
                stable_a.as_str(),
                "shared stable continuation",
                variant,
                stable_b.as_str(),
            ]
        })
        .collect::<Vec<_>>();
    let template = multi_episode_template(
        &rows,
        &[
            CoordinateKind::Stable,
            CoordinateKind::Slot,
            CoordinateKind::Stable,
            CoordinateKind::Stable,
            CoordinateKind::Slot,
            CoordinateKind::Stable,
        ],
    );
    let episodes = diagnose_template(&template, &DiagnosisOptions::default());

    assert_eq!(episodes.len(), 2);
    assert_eq!(episodes[0].episode_index, 1);
    assert_eq!(episodes[0].kind, EpisodeKind::Direct);
    assert_eq!(
        episodes[0].blocked_stable_bytes,
        stable_a.len() + "shared stable continuation".len()
    );
    assert_eq!(episodes[1].episode_index, 2);
    assert_eq!(episodes[1].kind, EpisodeKind::Conditional);
    assert_eq!(
        episodes[1].local_prefix_bytes,
        stable_a.len() + "shared stable continuation".len()
    );
    assert_eq!(episodes[1].blocked_stable_bytes, stable_b.len());
    assert_ne!(episodes[0].id, episodes[1].id);

    let mut reversed = template;
    reversed.members.reverse();
    reversed.member_maps.reverse();
    reversed.symbolic_members.reverse();
    assert_eq!(
        episodes,
        diagnose_template(&reversed, &DiagnosisOptions::default())
    );
}

#[test]
fn episode_budget_stops_deterministically() {
    let stable_a = "first stable recovery ".repeat(4);
    let stable_b = "second stable recovery ".repeat(4);
    let rows = ["north", "south", "east", "west"]
        .into_iter()
        .map(|variant| {
            vec![
                "prefix",
                variant,
                stable_a.as_str(),
                variant,
                stable_b.as_str(),
            ]
        })
        .collect::<Vec<_>>();
    let options = DiagnosisOptions {
        max_episodes_per_template: 1,
        ..DiagnosisOptions::default()
    };
    let episodes = diagnose_template(
        &multi_episode_template(
            &rows,
            &[
                CoordinateKind::Stable,
                CoordinateKind::Slot,
                CoordinateKind::Stable,
                CoordinateKind::Slot,
                CoordinateKind::Stable,
            ],
        ),
        &options,
    );
    assert_eq!(episodes.len(), 1);
    assert_eq!(episodes[0].kind, EpisodeKind::Direct);
}

#[test]
fn recovery_supporters_form_a_monotonic_chain() {
    let stable_a = "first stable recovery ".repeat(4);
    let stable_b = "second stable recovery ".repeat(4);
    let rows = ["north", "south", "east", "west", "minority"]
        .into_iter()
        .enumerate()
        .map(|(index, variant)| {
            vec![
                "prefix",
                variant,
                stable_a.as_str(),
                if index < 4 { variant } else { "excluded" },
                stable_b.as_str(),
            ]
        })
        .collect::<Vec<_>>();
    let mut value = multi_episode_template(
        &rows,
        &[
            CoordinateKind::Stable,
            CoordinateKind::Slot,
            CoordinateKind::Stable,
            CoordinateKind::Slot,
            CoordinateKind::Stable,
        ],
    );
    value.member_maps[4].bindings[2].state = CoordinateBindingState::Gap;
    let episodes = diagnose_template(&value, &DiagnosisOptions::default());

    assert_eq!(episodes.len(), 2);
    assert_eq!(episodes[0].comparable_count, 4);
    assert_eq!(episodes[1].comparable_count, 4);
}

#[test]
fn maximum_p2_tail_does_not_change_minimum_anchor_identity() {
    let anchor = "qualified stable anchor ".repeat(4);
    let first_rows = ["north", "south", "east", "west"]
        .into_iter()
        .map(|variant| vec!["prefix", variant, anchor.as_str(), "tail version one"])
        .collect::<Vec<_>>();
    let second_rows = ["north", "south", "east", "west"]
        .into_iter()
        .map(|variant| vec!["prefix", variant, anchor.as_str(), "tail version two"])
        .collect::<Vec<_>>();
    let kinds = [
        CoordinateKind::Stable,
        CoordinateKind::Slot,
        CoordinateKind::Stable,
        CoordinateKind::Stable,
    ];
    let first = diagnose_template(
        &multi_episode_template(&first_rows, &kinds),
        &DiagnosisOptions::default(),
    );
    let second = diagnose_template(
        &multi_episode_template(&second_rows, &kinds),
        &DiagnosisOptions::default(),
    );

    assert_eq!(first[0].id, second[0].id);
    assert!(first[0].blocked_stable_bytes > anchor.len());
}

fn coordinate(
    id: &str,
    ordinal: usize,
    kind: CoordinateKind,
    content: &str,
    bytes: usize,
    support: usize,
) -> TemplateCoordinate {
    TemplateCoordinate {
        id: TemplateCoordinateId(id.into()),
        ordinal,
        kind,
        logical_unit: LogicalUnitKey {
            collection: ContextCollectionKind::Messages,
            unit_kind: CacheUnitKind::VisibleText,
            alignment_key: format!("message:system:content:{ordinal}"),
            role: Some("system".into()),
            stable_identity: None,
        },
        medoid_unit_index: ordinal,
        medoid_utf8_bytes: 0..bytes,
        source: source(ordinal, bytes),
        content_digest: digest(content),
        observable_bytes: bytes,
        support_count: support,
        support_ratio: 1.0,
    }
}

fn binding(id: &str, unit_index: usize, content: &str) -> CoordinateBinding {
    CoordinateBinding {
        coordinate_id: TemplateCoordinateId(id.into()),
        state: CoordinateBindingState::Present(MemberFragmentRef {
            unit_index,
            utf8_bytes: Range {
                start: 0,
                end: content.len(),
            },
            source: source(unit_index, content.len()),
            observable_bytes: content.len(),
            content_digest: digest(content),
        }),
    }
}

fn source(index: usize, bytes: usize) -> SourceLocation {
    SourceLocation {
        json_path: format!("$.messages[{index}].content"),
        utf8_bytes: Some(0..bytes),
        message_index: Some(index),
        role: Some("system".into()),
    }
}

fn digest(value: &str) -> String {
    crate::diagnosis::identity::digest([value.as_bytes()])
}

fn domain() -> ComparisonDomain {
    ComparisonDomain {
        time_window_key: 0,
        endpoint_key: "endpoint".into(),
        model: "model".into(),
        model_deployment_key: None,
        context_schema_key: "schema".into(),
        agent_key: None,
        kv_namespace: None,
        dialect: "dialect".into(),
        adapter_revision: "revision".into(),
    }
}
