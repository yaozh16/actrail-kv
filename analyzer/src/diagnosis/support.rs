//! 本文件提供诊断引擎的 UTF-8 安全文本边界、稳定块匹配和证据摘要工具。

use std::collections::HashSet;

use sha2::{Digest, Sha256};

use super::{DiagnosisOptions, DiagnosticRequest, DiagnosticUnit, SourceLocation};

pub(super) fn same_unit(left: &DiagnosticUnit, right: &DiagnosticUnit) -> bool {
    left.role == right.role
        && left.stable_identity == right.stable_identity
        && left.content == right.content
}

pub(super) fn same_logical_unit(left: &DiagnosticUnit, right: &DiagnosticUnit) -> bool {
    left.role == right.role
        && left.stable_identity == right.stable_identity
        && left.ordinal == right.ordinal
}

fn same_shifted_unit(left: &DiagnosticUnit, right: &DiagnosticUnit) -> bool {
    left.role == right.role
        && left.stable_identity == right.stable_identity
        && left.content == right.content
}

pub(super) fn recoverable_after(
    position: usize,
    reference: &DiagnosticRequest,
    member: &DiagnosticRequest,
) -> usize {
    let mut bytes = 0;
    let mut left = reference.units.len();
    let mut right = member.units.len();
    while left > position + 1 && right > position + 1 {
        if !same_unit(&reference.units[left - 1], &member.units[right - 1]) {
            break;
        }
        bytes += reference.units[left - 1].content.len();
        left -= 1;
        right -= 1;
    }
    bytes
}

pub(super) fn shifted_stable_block(
    reference: &DiagnosticRequest,
    member: &DiagnosticRequest,
    first: usize,
) -> Option<(usize, usize, usize)> {
    let right_len = member.units.len();
    let mut previous_lengths = vec![0usize; right_len + 1];
    let mut previous_bytes = vec![0usize; right_len + 1];
    let mut best: Option<(usize, usize, usize)> = None;
    for left in first..reference.units.len() {
        let mut current_lengths = vec![0usize; right_len + 1];
        let mut current_bytes = vec![0usize; right_len + 1];
        for right in first..right_len {
            if left != right && same_shifted_unit(&reference.units[left], &member.units[right]) {
                let run_len = previous_lengths[right] + 1;
                let run_bytes = previous_bytes[right] + reference.units[left].content.len();
                current_lengths[right + 1] = run_len;
                current_bytes[right + 1] = run_bytes;
                let candidate = (left + 1 - run_len, right + 1 - run_len, run_bytes);
                if best.is_none_or(|prior| {
                    candidate.2 > prior.2
                        || (candidate.2 == prior.2
                            && (candidate.0, candidate.1) < (prior.0, prior.1))
                }) {
                    best = Some(candidate);
                }
            }
        }
        previous_lengths = current_lengths;
        previous_bytes = current_bytes;
    }
    best
}

pub(super) fn is_equal_length_reorder(
    reference: &DiagnosticRequest,
    member: &DiagnosticRequest,
) -> bool {
    if reference.units.len() != member.units.len() {
        return false;
    }
    let mut used = vec![false; member.units.len()];
    reference.units.iter().all(|left| {
        member
            .units
            .iter()
            .enumerate()
            .position(|(index, right)| !used[index] && same_shifted_unit(left, right))
            .is_some_and(|index| {
                used[index] = true;
                true
            })
    })
}

pub(super) fn recoverable_anchor_after<'a>(
    position: usize,
    reference: &'a DiagnosticRequest,
    member: &DiagnosticRequest,
    min_anchor_bytes: usize,
) -> Option<&'a str> {
    let mut left = reference.units.len();
    let mut right = member.units.len();
    while left > position + 1 && right > position + 1 {
        if !same_unit(&reference.units[left - 1], &member.units[right - 1]) {
            break;
        }
        left -= 1;
        right -= 1;
    }
    reference.units[left..]
        .iter()
        .map(|unit| unit.content.as_str())
        .find(|content| content.len() >= min_anchor_bytes)
}

pub(super) fn has_anchor_after(
    position: usize,
    reference: &DiagnosticRequest,
    member: &DiagnosticRequest,
    options: &DiagnosisOptions,
) -> bool {
    let mut right_seen = HashSet::new();
    for unit in member.units.iter().skip(position + 1) {
        if unit.content.len() >= options.min_anchor_bytes {
            right_seen.insert((&unit.role, &unit.content));
        }
    }
    reference.units.iter().skip(position + 1).any(|unit| {
        unit.content.len() >= options.min_anchor_bytes
            && right_seen.contains(&(&unit.role, &unit.content))
    })
}

pub(super) fn shifted_anchor<'a>(
    reference: &'a DiagnosticRequest,
    member: &DiagnosticRequest,
    left_start: usize,
    right_start: usize,
    min_anchor_bytes: usize,
) -> Option<&'a str> {
    reference
        .units
        .iter()
        .skip(left_start)
        .zip(member.units.iter().skip(right_start))
        .take_while(|(left, right)| same_shifted_unit(left, right))
        .map(|(left, _)| left.content.as_str())
        .find(|content| content.len() >= min_anchor_bytes)
}

pub(super) fn prefix_bytes(units: &[DiagnosticUnit], end: usize) -> usize {
    units.iter().take(end).map(|unit| unit.content.len()).sum()
}

pub(super) fn common_prefix_boundary(left: &str, right: &str) -> usize {
    left.char_indices()
        .zip(right.char_indices())
        .take_while(|((_, a), (_, b))| a == b)
        .map(|((index, ch), _)| index + ch.len_utf8())
        .last()
        .unwrap_or(0)
}

pub(super) fn common_suffix_boundary(left: &str, right: &str) -> usize {
    left.chars()
        .rev()
        .zip(right.chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(ch, _)| ch.len_utf8())
        .sum()
}

pub(super) fn ranged_source(unit: &DiagnosticUnit, local_offset: usize) -> SourceLocation {
    let utf8_range = unit.source.utf8_range.map(|(start, end)| {
        let point = (start + local_offset).min(end);
        (point, point)
    });
    SourceLocation {
        json_path: unit.source.json_path.clone(),
        utf8_range,
    }
}

pub(super) fn normalized_position(position: usize) -> String {
    format!("unit:{position}")
}

pub(super) fn abbreviated_anchor(content: &str) -> String {
    content.chars().take(48).collect()
}

pub(super) fn digest(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}
