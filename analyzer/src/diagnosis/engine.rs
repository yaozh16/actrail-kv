//! 本文件实现与业务字段和值无关的首分歧、稳定后缀和八类根因诊断。

use std::collections::{BTreeMap, HashMap};

use super::support::*;
use super::{
    Cause, DiagnosisObservation, DiagnosisOptions, DiagnosticRequest, DiagnosticTemplate,
    DiagnosticUnit, SourceLocation, UnitRole,
};
use crate::discovery::candidate::bounded_text_similarity;

#[derive(Debug)]
struct PairDiagnosis {
    cause: Cause,
    position: usize,
    inner_prefix: usize,
    blocked: usize,
    anchor: String,
    source: SourceLocation,
    evidence: String,
    counterfactual: String,
    recommendation: String,
    variant: String,
}

pub fn diagnose_template(
    template: &DiagnosticTemplate,
    options: &DiagnosisOptions,
) -> Vec<DiagnosisObservation> {
    let Some(reference) = template.members.iter().min_by(|left, right| {
        left.units
            .len()
            .cmp(&right.units.len())
            .then_with(|| left.request_id.cmp(&right.request_id))
    }) else {
        return Vec::new();
    };
    if template.members.len() < options.min_stable_support {
        return Vec::new();
    }

    let mut candidates = Vec::new();
    for member in &template.members {
        if member.request_id == reference.request_id {
            continue;
        }
        if let Some(pair) = diagnose_pair(reference, member, options) {
            let actual = prefix_bytes(&reference.units, pair.position) + pair.inner_prefix;
            candidates.push((member, pair, actual));
        }
    }

    // Stable span support is evaluated across members at a normalized location/cause, not per pair.
    let mut support = HashMap::<(Cause, String), usize>::new();
    for (_, pair, _) in &candidates {
        *support
            .entry((pair.cause.clone(), normalized_position(pair.position)))
            .or_default() += 1;
    }
    let baseline_support = template
        .members
        .iter()
        .filter(|member| member.units == reference.units)
        .count();
    candidates
        .into_iter()
        .filter(|(_, pair, _)| {
            let matched = support[&(pair.cause.clone(), normalized_position(pair.position))]
                + baseline_support;
            matched >= options.min_stable_support
                && matched as f64 / template.members.len() as f64 >= options.stable_support_rate
        })
        .map(|(member, pair, actual)| DiagnosisObservation {
            template_id: template.template_id.clone(),
            template_member_count: template.members.len(),
            member_id: member.request_id.clone(),
            cause: pair.cause,
            normalized_position: normalized_position(pair.position),
            stable_anchor: pair.anchor,
            actual_prefix_bytes: actual,
            potential_prefix_bytes: actual + pair.blocked,
            blocked_stable_bytes: pair.blocked,
            confidence: (template.cohesion * template.projection_reliability).clamp(0.0, 1.0),
            source: pair.source,
            evidence: pair.evidence,
            counterfactual: pair.counterfactual,
            recommendation: pair.recommendation,
            variant_fingerprint: pair.variant,
        })
        .collect()
}

fn diagnose_pair(
    reference: &DiagnosticRequest,
    member: &DiagnosticRequest,
    options: &DiagnosisOptions,
) -> Option<PairDiagnosis> {
    let first = reference
        .units
        .iter()
        .zip(&member.units)
        .position(|(left, right)| !same_unit(left, right))
        .unwrap_or(reference.units.len().min(member.units.len()));
    if first == reference.units.len() && first == member.units.len() {
        return None;
    }

    if let Some(result) = tool_reorder(reference, member, first, options) {
        return Some(result);
    }

    let left = reference.units.get(first);
    let right = member.units.get(first);
    if let (Some(left), Some(right)) = (left, right) {
        if let Some(result) =
            visible_json_formatting(left, right, first, reference, member, options)
        {
            return Some(result);
        }
        if left.role == UnitRole::ToolDefinition
            && right.role == UnitRole::ToolDefinition
            && left.stable_identity.is_some()
            && left.stable_identity == right.stable_identity
            && bounded_text_similarity(&left.content, &right.content)
                >= options.local_text_similarity
        {
            return text_result(
                Cause::ToolDefinitionDrift,
                first,
                left,
                right,
                reference,
                member,
                options,
                "保持同一工具定义的模型可见序列化完全稳定",
            );
        }
        if left.role == UnitRole::SystemPrompt
            && right.role == UnitRole::SystemPrompt
            && bounded_text_similarity(&left.content, &right.content)
                >= options.local_text_similarity
        {
            return text_result(
                Cause::PromptMicroDrift,
                first,
                left,
                right,
                reference,
                member,
                options,
                "固定系统提示词版本，变化内容移至稳定前缀之后",
            );
        }
        if left.role == UnitRole::Conversation && right.role == UnitRole::Conversation {
            let shifted = shifted_stable_block(reference, member, first).filter(
                |(left_index, right_index, _)| {
                    reference.units[*left_index].role == UnitRole::Conversation
                        && member.units[*right_index].role == UnitRole::Conversation
                },
            );
            let blocked = recoverable_after(first, reference, member)
                .max(shifted.map_or(0, |(_, _, bytes)| bytes));
            if blocked >= options.min_blocked_bytes
                && (has_anchor_after(first, reference, member, options) || shifted.is_some())
            {
                let anchor = shifted
                    .and_then(|(left_index, right_index, _)| {
                        shifted_anchor(
                            reference,
                            member,
                            left_index,
                            right_index,
                            options.min_anchor_bytes,
                        )
                    })
                    .or_else(|| {
                        recoverable_anchor_after(first, reference, member, options.min_anchor_bytes)
                    })?;
                let mut result = make_result(
                    Cause::NonAppendHistory,
                    first,
                    0,
                    blocked,
                    right,
                    anchor,
                    "历史消息在已有稳定历史之前发生替换或插入",
                    "仅在历史尾部追加新消息可恢复后续稳定前缀",
                    "保持历史只追加；不要在已发送历史中间改写或插入消息",
                );
                if let Some((_, right_index, _)) = shifted {
                    result.variant = variant_span_digest(member, first, right_index.max(first + 1));
                }
                return Some(result);
            }
        }
        let inline_prefix = common_prefix_boundary(&left.content, &right.content);
        let inline_suffix = common_suffix_boundary(
            &left.content[inline_prefix..],
            &right.content[inline_prefix..],
        );
        if same_logical_unit(left, right)
            && (inline_prefix >= options.min_anchor_bytes
                || inline_suffix >= options.min_anchor_bytes)
        {
            if let Some(result) = text_result(
                Cause::InlineDynamicSlot,
                first,
                left,
                right,
                reference,
                member,
                options,
                "把单元内部变化槽移到稳定文本之后或拆为后置单元",
            ) {
                return Some(result);
            }
        }
    }

    if let Some((left_index, right_index, blocked)) = shifted_stable_block(reference, member, first)
    {
        if blocked >= options.min_blocked_bytes
            && shifted_anchor(
                reference,
                member,
                left_index,
                right_index,
                options.min_anchor_bytes,
            )
            .is_some()
            && !(reference.units[left_index].role != UnitRole::Conversation
                && is_equal_length_reorder(reference, member))
        {
            let source_unit = member.units.get(first).or(left)?;
            let anchor = shifted_anchor(
                reference,
                member,
                left_index,
                right_index,
                options.min_anchor_bytes,
            )?;
            let mut result = make_result(
                Cause::DynamicBlockBeforeStatic,
                first,
                0,
                blocked,
                source_unit,
                anchor,
                &format!("稳定块位置从 {left_index} 漂移到 {right_index}"),
                "将变化块后移可使稳定块重新连续",
                "把动态块放在稳定指令、工具或历史块之后",
            );
            result.variant = variant_span_digest(member, first, right_index.max(first + 1));
            return Some(result);
        }
    }

    let blocked = recoverable_after(first, reference, member);
    if blocked >= options.min_blocked_bytes && has_anchor_after(first, reference, member, options) {
        let source_unit = member.units.get(first).or(left)?;
        let anchor = recoverable_anchor_after(first, reference, member, options.min_anchor_bytes)?;
        return Some(make_result(
            Cause::EarlyVolatileContent,
            first,
            0,
            blocked,
            source_unit,
            anchor,
            "首个变化出现在后续稳定内容之前",
            "后移或固定该变化可恢复后续稳定前缀",
            "将早期易变内容移至稳定上下文之后",
        ));
    }
    // A differing suffix has no blocked stable span and is the expected append-only pattern.
    None
}

fn tool_reorder(
    reference: &DiagnosticRequest,
    member: &DiagnosticRequest,
    first: usize,
    options: &DiagnosisOptions,
) -> Option<PairDiagnosis> {
    let left_tools = tool_run(&reference.units, first);
    let right_tools = tool_run(&member.units, first);
    if left_tools.len() < 2 || left_tools.len() != right_tools.len() {
        return None;
    }
    let left_map = unique_tool_map(left_tools)?;
    let right_map = unique_tool_map(right_tools)?;
    if left_map != right_map {
        return None;
    }
    let left_order: Vec<_> = left_tools
        .iter()
        .map(|u| u.stable_identity.as_deref())
        .collect();
    let right_order: Vec<_> = right_tools
        .iter()
        .map(|u| u.stable_identity.as_deref())
        .collect();
    if left_order == right_order {
        return None;
    }
    let after = first + left_tools.len();
    let blocked = recoverable_after(after.saturating_sub(1), reference, member)
        + left_tools
            .iter()
            .map(|unit| unit.content.len())
            .sum::<usize>();
    if blocked < options.min_blocked_bytes {
        return None;
    }
    Some(make_result(
        Cause::ToolOrderDrift,
        first,
        0,
        blocked,
        &right_tools[0],
        &left_tools[0].content,
        "唯一工具身份和定义集合相同，但模型可见顺序不同",
        "按同一稳定顺序排列工具可恢复整个工具块",
        "按稳定工具身份排序，或由调用方固定注册顺序",
    ))
}

fn unique_tool_map(units: &[DiagnosticUnit]) -> Option<BTreeMap<&str, &str>> {
    let mut map = BTreeMap::new();
    for unit in units {
        let identity = unit.stable_identity.as_deref()?;
        if map.insert(identity, unit.content.as_str()).is_some() {
            return None;
        }
    }
    Some(map)
}

fn tool_run(units: &[DiagnosticUnit], start: usize) -> &[DiagnosticUnit] {
    let end = units[start..]
        .iter()
        .position(|unit| unit.role != UnitRole::ToolDefinition)
        .map_or(units.len(), |offset| start + offset);
    &units[start..end]
}

fn visible_json_formatting(
    left: &DiagnosticUnit,
    right: &DiagnosticUnit,
    position: usize,
    reference: &DiagnosticRequest,
    member: &DiagnosticRequest,
    options: &DiagnosisOptions,
) -> Option<PairDiagnosis> {
    let is_visible_text = |role: &UnitRole| {
        matches!(
            role,
            UnitRole::SystemPrompt | UnitRole::Conversation | UnitRole::ModelVisibleText
        )
    };
    if !is_visible_text(&left.role) || !is_visible_text(&right.role) {
        return None;
    }
    let left_json: serde_json::Value = serde_json::from_str(&left.content).ok()?;
    let right_json: serde_json::Value = serde_json::from_str(&right.content).ok()?;
    if left_json != right_json || left.content == right.content {
        return None;
    }
    text_result(
        Cause::ModelVisibleJsonFormattingDrift,
        position,
        left,
        right,
        reference,
        member,
        options,
        "对模型可见 JSON 使用确定性的键序、空白和转义格式",
    )
}

#[allow(clippy::too_many_arguments)]
fn text_result(
    cause: Cause,
    position: usize,
    left: &DiagnosticUnit,
    right: &DiagnosticUnit,
    reference: &DiagnosticRequest,
    member: &DiagnosticRequest,
    options: &DiagnosisOptions,
    recommendation: &str,
) -> Option<PairDiagnosis> {
    let prefix = common_prefix_boundary(&left.content, &right.content);
    let suffix = common_suffix_boundary(&left.content[prefix..], &right.content[prefix..]);
    let blocked = suffix + recoverable_after(position, reference, member);
    if (cause == Cause::InlineDynamicSlot && (prefix == 0 || suffix < options.min_anchor_bytes))
        || blocked < options.min_blocked_bytes
        || (suffix < options.min_anchor_bytes
            && !has_anchor_after(position, reference, member, options))
    {
        return None;
    }
    let anchor = if suffix >= options.min_anchor_bytes {
        &left.content[left.content.len() - suffix..]
    } else {
        recoverable_anchor_after(position, reference, member, options.min_anchor_bytes)?
    };
    Some(make_result(
        cause,
        position,
        prefix,
        blocked,
        right,
        anchor,
        "同一逻辑单元在局部变化后仍有稳定模型可见内容",
        "固定或后移局部变化可连接前缀与后续稳定内容",
        recommendation,
    ))
}

#[allow(clippy::too_many_arguments)]
fn make_result(
    cause: Cause,
    position: usize,
    inner_prefix: usize,
    blocked: usize,
    source_unit: &DiagnosticUnit,
    stable_anchor: &str,
    evidence: &str,
    counterfactual: &str,
    recommendation: &str,
) -> PairDiagnosis {
    let anchor = abbreviated_anchor(stable_anchor);
    PairDiagnosis {
        cause,
        position,
        inner_prefix,
        blocked,
        anchor,
        source: ranged_source(source_unit, inner_prefix),
        evidence: format!(
            "{evidence}; observed variant: {}",
            abbreviated_anchor(&source_unit.content)
        ),
        counterfactual: counterfactual.to_owned(),
        recommendation: recommendation.to_owned(),
        variant: digest(&format!("{:?}\0{}", source_unit.role, source_unit.content)),
    }
}

fn variant_span_digest(request: &DiagnosticRequest, start: usize, end: usize) -> String {
    let material = request.units[start..end.min(request.units.len())]
        .iter()
        .map(|unit| format!("{:?}\0{}", unit.role, unit.content))
        .collect::<Vec<_>>()
        .join("\u{1f}");
    digest(&material)
}

#[cfg(test)]
mod tests;
