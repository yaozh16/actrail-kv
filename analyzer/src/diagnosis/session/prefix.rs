//! 本文件在开放投影视图上计算单一 Session 前缀边界。

use actrail_kv_artifacts::{
    SessionDivergence, SessionPrefixMetrics, SessionTransitionOutcome,
    SourceLocation as ArtifactSource,
};

use crate::model::projection::{CacheSequence, CacheUnit};

pub fn compare(previous: &CacheSequence, current: &CacheSequence) -> SessionTransitionOutcome {
    let previous_units = open_units(previous);
    let current_units = open_units(current);
    let previous_bytes = observable_bytes(&previous_units);
    let current_bytes = observable_bytes(&current_units);
    let mut preserved = 0usize;
    let common_units = previous_units.len().min(current_units.len());

    for index in 0..common_units {
        let left = previous_units[index];
        let right = current_units[index];
        if same_unit_shape(left, right) && left.content == right.content {
            preserved += left.observable_bytes();
            continue;
        }
        let shapes_match = same_unit_shape(left, right);
        let common = if shapes_match {
            utf8_common_prefix(&left.content, &right.content)
        } else {
            0
        };
        preserved += common;
        if shapes_match && common == right.content.len() && index + 1 == current_units.len() {
            return SessionTransitionOutcome::PrefixTruncated {
                metrics: metrics(previous_bytes, current_bytes, preserved),
                divergence: divergence(Some(left), Some(right), index, common),
            };
        }
        return SessionTransitionOutcome::HistoryChanged {
            metrics: metrics(previous_bytes, current_bytes, preserved),
            divergence: divergence(Some(left), Some(right), index, common),
        };
    }

    if previous_units.len() == current_units.len() {
        SessionTransitionOutcome::Identical {
            metrics: metrics(previous_bytes, current_bytes, preserved),
        }
    } else if previous_units.len() < current_units.len() {
        SessionTransitionOutcome::NormalAppend {
            metrics: metrics(previous_bytes, current_bytes, preserved),
        }
    } else {
        SessionTransitionOutcome::PrefixTruncated {
            metrics: metrics(previous_bytes, current_bytes, preserved),
            divergence: divergence(
                previous_units.get(common_units).copied(),
                None,
                common_units,
                0,
            ),
        }
    }
}

fn open_units(sequence: &CacheSequence) -> Vec<&CacheUnit> {
    sequence
        .units
        .iter()
        .filter(|unit| !is_collection_terminal(unit))
        .collect()
}

fn is_collection_terminal(unit: &CacheUnit) -> bool {
    use crate::model::projection::{CacheUnitKind, ContextCollectionKind};

    if unit.kind != CacheUnitKind::Structural || unit.content != unit.alignment_key {
        return false;
    }
    matches!(
        (
            unit.alignment_key.as_str(),
            &unit.hierarchy.collection,
            unit.hierarchy.parent_json_path.as_str(),
            unit.source.json_path.as_str(),
        ),
        (
            "messages:end",
            ContextCollectionKind::Messages,
            "$.messages",
            "$.messages"
        ) | (
            "tools:end",
            ContextCollectionKind::Tools,
            "$.tools",
            "$.tools"
        )
    )
}

fn observable_bytes(units: &[&CacheUnit]) -> usize {
    units.iter().map(|unit| unit.observable_bytes()).sum()
}

fn same_unit_shape(left: &CacheUnit, right: &CacheUnit) -> bool {
    left.kind == right.kind
        && left.alignment_key == right.alignment_key
        && left.hierarchy == right.hierarchy
}

fn utf8_common_prefix(left: &str, right: &str) -> usize {
    let bytes = left.as_bytes();
    let other = right.as_bytes();
    let mut end = bytes
        .iter()
        .zip(other)
        .take_while(|(left, right)| left == right)
        .count();
    while end > 0 && (!left.is_char_boundary(end) || !right.is_char_boundary(end)) {
        end -= 1;
    }
    end
}

fn metrics(previous: usize, current: usize, preserved: usize) -> SessionPrefixMetrics {
    SessionPrefixMetrics {
        previous_observable_bytes: previous,
        current_observable_bytes: current,
        preserved_prefix_bytes: preserved,
        prefix_retention_ratio: if previous == 0 {
            1.0
        } else {
            preserved as f64 / previous as f64
        },
        invalidated_previous_suffix_bytes: previous.saturating_sub(preserved),
    }
}

fn divergence(
    previous: Option<&CacheUnit>,
    current: Option<&CacheUnit>,
    unit_index: usize,
    byte_offset: usize,
) -> SessionDivergence {
    SessionDivergence {
        previous_source: previous.map(|unit| source(unit, unit_index, byte_offset)),
        current_source: current.map(|unit| source(unit, unit_index, byte_offset)),
        previous_excerpt: previous.map(|unit| excerpt(&unit.content, byte_offset)),
        current_excerpt: current.map(|unit| excerpt(&unit.content, byte_offset)),
        logical_position: logical_position(previous.or(current).expect("one side diverges")),
    }
}

fn source(unit: &CacheUnit, unit_index: usize, byte_offset: usize) -> ArtifactSource {
    let source = &unit.source;
    ArtifactSource {
        json_path: source.json_path.clone(),
        logical_scope: source.role.iter().cloned().collect(),
        unit_index: Some(unit_index),
        byte_start: Some(byte_offset),
        byte_end: Some(unit.content.len()),
        role: source.role.clone(),
    }
}

fn logical_position(unit: &CacheUnit) -> String {
    format!(
        "{:?}/{}/{:?}/{}/{:?}/{:?}/{}",
        unit.kind,
        unit.alignment_key,
        unit.hierarchy.collection,
        unit.hierarchy.parent_json_path,
        unit.hierarchy.element_index,
        unit.hierarchy.content_block_index,
        unit.source.role.as_deref().unwrap_or("none")
    )
}

fn excerpt(value: &str, byte_offset: usize) -> String {
    let start = value[..byte_offset]
        .char_indices()
        .rev()
        .nth(31)
        .map_or(0, |v| v.0);
    value[start..].chars().take(96).collect()
}

#[cfg(test)]
pub(super) fn source_location(path: &str) -> crate::model::projection::SourceLocation {
    crate::model::projection::SourceLocation {
        json_path: path.to_owned(),
        utf8_bytes: None,
        message_index: None,
        role: None,
    }
}
