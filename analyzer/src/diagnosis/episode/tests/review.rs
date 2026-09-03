//! 本文件覆盖多 episode review 发现的状态提交、稳定恢复与身份反例。

use super::*;

#[test]
fn default_budget_caps_a_template_at_four_episodes() {
    let stable_a = "stable section a ".repeat(5);
    let stable_b = "stable section b ".repeat(5);
    let stable_c = "stable section c ".repeat(5);
    let stable_d = "stable section d ".repeat(5);
    let stable_e = "stable section e ".repeat(5);
    let rows = ["north", "south", "east", "west"]
        .into_iter()
        .map(|variant| {
            vec![
                "prefix",
                variant,
                stable_a.as_str(),
                variant,
                stable_b.as_str(),
                variant,
                stable_c.as_str(),
                variant,
                stable_d.as_str(),
                variant,
                stable_e.as_str(),
            ]
        })
        .collect::<Vec<_>>();
    let episodes = diagnose_template(
        &multi_episode_template(
            &rows,
            &[
                CoordinateKind::Stable,
                CoordinateKind::Slot,
                CoordinateKind::Stable,
                CoordinateKind::Slot,
                CoordinateKind::Stable,
                CoordinateKind::Slot,
                CoordinateKind::Stable,
                CoordinateKind::Slot,
                CoordinateKind::Stable,
                CoordinateKind::Slot,
                CoordinateKind::Stable,
            ],
        ),
        &DiagnosisOptions::default(),
    );

    assert_eq!(episodes.len(), 4);
    assert_eq!(
        episodes
            .iter()
            .map(|episode| episode.episode_index)
            .collect::<Vec<_>>(),
        [1, 2, 3, 4]
    );
}

#[test]
fn rejected_early_region_does_not_consume_direct_number_or_supporters() {
    let stable_a = "first recovery ".repeat(5);
    let stable_b = "second recovery ".repeat(5);
    let rows = ["north", "south", "east", "west", "minority"]
        .into_iter()
        .enumerate()
        .map(|(index, variant)| {
            vec![
                "prefix",
                if index < 4 { "same" } else { "odd" },
                stable_a.as_str(),
                variant,
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
    assert_eq!(episodes.len(), 1);
    assert_eq!(episodes[0].kind, EpisodeKind::Direct);
    assert_eq!(episodes[0].episode_index, 1);
    assert_eq!(episodes[0].comparable_count, 5);
    assert_eq!(episodes[0].blocked_stable_bytes, stable_b.len());
}

#[test]
fn identical_unmatched_runs_are_part_of_maximum_p2_not_a_new_mismatch() {
    let stable_a = "stable before insertion ".repeat(4);
    let stable_b = "stable after insertion ".repeat(4);
    let stable_c = "later recovery ".repeat(5);
    let inserted = "shared unmatched fragment";
    let rows = ["north", "south", "east", "west"]
        .into_iter()
        .map(|variant| {
            vec![
                "prefix",
                variant,
                stable_a.as_str(),
                stable_b.as_str(),
                variant,
                stable_c.as_str(),
            ]
        })
        .collect::<Vec<_>>();
    let mut value = multi_episode_template(
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
    for member in &mut value.members {
        member.units.push(CacheUnit {
            kind: CacheUnitKind::VisibleText,
            alignment_key: "unmatched:shared".into(),
            content: inserted.into(),
            source: source(6, inserted.len()),
            tool_identity: None,
            hierarchy: HierarchyLocation {
                collection: ContextCollectionKind::Messages,
                parent_json_path: "$.messages".into(),
                element_index: Some(6),
                content_block_index: None,
            },
        });
    }
    for map in &mut value.member_maps {
        map.unmatched_runs.push(UnmatchedRun {
            run_id: "shared-run".into(),
            left_coordinate_id: Some(TemplateCoordinateId("coordinate-2".into())),
            right_coordinate_id: Some(TemplateCoordinateId("coordinate-3".into())),
            fragments: vec![MemberFragmentRef {
                unit_index: 6,
                utf8_bytes: 0..inserted.len(),
                source: source(6, inserted.len()),
                observable_bytes: inserted.len(),
                content_digest: digest(inserted),
            }],
        });
    }

    let episodes = diagnose_template(&value, &DiagnosisOptions::default());
    assert_eq!(episodes.len(), 2);
    assert_eq!(
        episodes[0].blocked_stable_bytes,
        stable_a.len() + inserted.len() + stable_b.len()
    );
}

#[test]
fn active_equal_slot_extends_recovery_after_minority_supporter_drops() {
    let stable_a = "stable recovery start ".repeat(4);
    let stable_b = "stable recovery end ".repeat(4);
    let later = "later recovery ".repeat(5);
    let rows = ["north", "south", "east", "west", "minority"]
        .into_iter()
        .enumerate()
        .map(|(index, variant)| {
            vec![
                "prefix",
                variant,
                stable_a.as_str(),
                if index < 4 {
                    "active-shared"
                } else {
                    "minority-only"
                },
                stable_b.as_str(),
                variant,
                later.as_str(),
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
            CoordinateKind::Slot,
            CoordinateKind::Stable,
        ],
    );
    value.member_maps[4].bindings[2].state = CoordinateBindingState::Gap;

    let episodes = diagnose_template(&value, &DiagnosisOptions::default());
    assert_eq!(episodes.len(), 2);
    assert_eq!(episodes[0].comparable_count, 4);
    assert_eq!(
        episodes[0].blocked_stable_bytes,
        stable_a.len() + "active-shared".len() + stable_b.len()
    );
    assert_eq!(episodes[1].episode_index, 2);
}

#[test]
fn conditional_identity_is_template_local_while_direct_identity_stays_logical() {
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
    let kinds = [
        CoordinateKind::Stable,
        CoordinateKind::Slot,
        CoordinateKind::Stable,
        CoordinateKind::Slot,
        CoordinateKind::Stable,
    ];
    let first = multi_episode_template(&rows, &kinds);
    let mut second = first.clone();
    second.id = "another-template".into();
    let first = diagnose_template(&first, &DiagnosisOptions::default());
    let second = diagnose_template(&second, &DiagnosisOptions::default());

    assert_eq!(first[0].id, second[0].id);
    assert_ne!(first[1].id, second[1].id);
}
