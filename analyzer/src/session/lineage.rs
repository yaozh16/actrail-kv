//! 按 session 相邻对计算公共前缀、切换类型与可恢复稳定字节。

use std::collections::{BTreeMap, HashMap};

use actrail_kv_artifacts::{
    ContextDefect, PrefixReuseComposition, PrefixReuseRequest, PrefixReuseSession,
    PrefixReuseSummary, SessionEventType, SessionReport, SessionSwitchEvent,
};
use sha2::{Digest, Sha256};

use crate::model::projection::{CacheSequence, CacheUnit, CacheUnitKind, ContextCollectionKind};

/// 会话内一行：session 标识与对应投影序列。
#[derive(Clone, Debug)]
pub(crate) struct SessionRow<'a> {
    pub session_key: &'a str,
    pub sequence: &'a CacheSequence,
}

/// 按会话与比较域分组，输出按“切换后可恢复稳定字节”降序的报告。
pub(crate) fn analyze_reports(rows: Vec<SessionRow<'_>>) -> Vec<SessionReport> {
    let mut buckets: Vec<(String, String, String, Vec<&CacheSequence>)> = Vec::new();
    let mut by_key: HashMap<(String, String, String), usize> = HashMap::new();
    for row in rows {
        let domain = &row.sequence.domain;
        let key = (
            row.session_key.to_owned(),
            domain.endpoint_key.clone(),
            domain.model.clone(),
        );
        let index = match by_key.get(&key) {
            Some(index) => *index,
            None => {
                let index = buckets.len();
                by_key.insert(key.clone(), index);
                buckets.push((key.0, key.1, key.2, Vec::new()));
                index
            }
        };
        buckets[index].3.push(row.sequence);
    }

    let mut reports: Vec<_> = buckets
        .into_iter()
        .filter(|(_, _, _, sequences)| sequences.len() >= 2)
        .map(|(session_key, endpoint_key, model, sequences)| {
            build_report(session_key, endpoint_key, model, sequences)
        })
        .collect();
    reports.sort_by(|left, right| {
        right
            .total_stable_after_switch_bytes
            .cmp(&left.total_stable_after_switch_bytes)
            .then_with(|| left.session_key.cmp(&right.session_key))
    });
    reports
}

fn build_report(
    session_key: String,
    endpoint_key: String,
    model: String,
    sequences: Vec<&CacheSequence>,
) -> SessionReport {
    let mut events = Vec::new();
    let mut append_count = 0usize;
    let mut fork_count = 0usize;
    let mut reorder_count = 0usize;
    let mut reset_count = 0usize;
    let mut total_recomputed_bytes = 0usize;
    let mut total_stable_after_switch_bytes = 0usize;
    let mut total_prefix_cut_bytes = 0usize;
    let mut previous_lcp_bytes: Option<usize> = None;
    for pair in sequences.windows(2) {
        let (event_type, lcp_units, lcp_bytes) = classify(pair[0], pair[1]);
        let previous_pair_lcp_bytes = previous_lcp_bytes.unwrap_or(0);
        let prefix_cut_bytes = previous_pair_lcp_bytes.saturating_sub(lcp_bytes);
        previous_lcp_bytes = Some(lcp_bytes);
        let next_total_bytes = observable_bytes(&pair[1].units);
        let recomputed_bytes = next_total_bytes.saturating_sub(lcp_bytes);
        let stable_after_switch_bytes = recovered_contiguous_bytes(pair[0], pair[1], lcp_units);
        match event_type {
            SessionEventType::Append => append_count += 1,
            SessionEventType::Fork => fork_count += 1,
            SessionEventType::Reorder => reorder_count += 1,
            SessionEventType::Reset => reset_count += 1,
        }
        total_recomputed_bytes = total_recomputed_bytes.saturating_add(recomputed_bytes);
        total_stable_after_switch_bytes =
            total_stable_after_switch_bytes.saturating_add(stable_after_switch_bytes);
        total_prefix_cut_bytes = total_prefix_cut_bytes.saturating_add(prefix_cut_bytes);
        events.push(SessionSwitchEvent {
            event_type,
            prev_request_id: pair[0].request_id.clone(),
            next_request_id: pair[1].request_id.clone(),
            lcp_units,
            lcp_bytes,
            previous_pair_lcp_bytes,
            prefix_cut_bytes,
            next_total_bytes,
            recomputed_bytes,
            stable_after_switch_bytes,
            next_block_runs: run_length_summary(&pair[1].units),
        });
    }
    let event_count = events.len();
    SessionReport {
        session_key,
        endpoint_key,
        model,
        request_count: sequences.len(),
        append_count,
        fork_count,
        reorder_count,
        reset_count,
        total_recomputed_bytes,
        total_stable_after_switch_bytes,
        total_prefix_cut_bytes,
        avg_prefix_cut_bytes: if event_count == 0 {
            0
        } else {
            total_prefix_cut_bytes / event_count
        },
        events,
    }
}

fn run_length_summary(units: &[CacheUnit]) -> String {
    let mut runs = Vec::new();
    let mut current: Option<(String, usize)> = None;
    for unit in units.iter().filter(|unit| is_content_bearing(unit)) {
        let digest = unit_digest(unit);
        match &mut current {
            Some((token, count)) if *token == digest => *count += 1,
            Some((token, count)) => {
                runs.push(format!("{token}x{count}"));
                current = Some((digest, 1));
            }
            None => current = Some((digest, 1)),
        }
    }
    if let Some((token, count)) = current {
        runs.push(format!("{token}x{count}"));
    }
    runs.join("/")
}

fn unit_digest(unit: &CacheUnit) -> String {
    let digest = Sha256::digest(unit.content.as_bytes());
    digest
        .iter()
        .take(4)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn classify(prev: &CacheSequence, next: &CacheSequence) -> (SessionEventType, usize, usize) {
    let mut lcp = 0usize;
    while lcp < prev.units.len()
        && lcp < next.units.len()
        && same_observable(&prev.units[lcp], &next.units[lcp])
    {
        lcp += 1;
    }
    let lcp_bytes = prev.units[..lcp]
        .iter()
        .map(|unit| unit.content.len())
        .sum();
    let prev_content_len = prev
        .units
        .iter()
        .filter(|unit| is_content_bearing(unit))
        .count();
    let next_content_len = next
        .units
        .iter()
        .filter(|unit| is_content_bearing(unit))
        .count();
    if lcp == prev.units.len()
        || (content_prefix_units(&prev.units, &next.units) == prev_content_len
            && next_content_len > prev_content_len)
    {
        return (SessionEventType::Append, lcp, lcp_bytes);
    }
    if unique_permutation(&prev.units, &next.units) {
        return (SessionEventType::Reorder, lcp, lcp_bytes);
    }
    if content_prefix_units(&prev.units, &next.units) == 0 {
        (SessionEventType::Reset, lcp, lcp_bytes)
    } else {
        (SessionEventType::Fork, lcp, lcp_bytes)
    }
}

/// 内容性单位从请求起始连续相同的个数（结构/role 标记不参与 reset 判定）。
fn content_prefix_units(prev: &[CacheUnit], next: &[CacheUnit]) -> usize {
    let prev_content: Vec<_> = prev
        .iter()
        .filter(|unit| is_content_bearing(unit))
        .collect();
    let next_content: Vec<_> = next
        .iter()
        .filter(|unit| is_content_bearing(unit))
        .collect();
    let mut count = 0usize;
    while count < prev_content.len()
        && count < next_content.len()
        && same_observable(prev_content[count], next_content[count])
    {
        count += 1;
    }
    count
}

fn same_observable(left: &CacheUnit, right: &CacheUnit) -> bool {
    left.kind == right.kind
        && left.alignment_key == right.alignment_key
        && left.content == right.content
        && left.hierarchy.collection == right.hierarchy.collection
}

/// 仅在内容性单位多重集相同且各自单位唯一时声称 reorder（role/结构噪声不参与）。
fn unique_permutation(prev: &[CacheUnit], next: &[CacheUnit]) -> bool {
    let prev_content: Vec<_> = prev
        .iter()
        .filter(|unit| is_content_bearing(unit))
        .collect();
    let next_content: Vec<_> = next
        .iter()
        .filter(|unit| is_content_bearing(unit))
        .collect();
    if prev_content.len() != next_content.len() || prev_content.is_empty() {
        return false;
    }
    let mut prev_counts: BTreeMap<UnitFingerprint, usize> = BTreeMap::new();
    let mut next_counts: BTreeMap<UnitFingerprint, usize> = BTreeMap::new();
    for unit in &prev_content {
        prev_counts
            .entry(fingerprint(unit))
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }
    for unit in &next_content {
        next_counts
            .entry(fingerprint(unit))
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }
    prev_counts.len() == prev_content.len() && prev_counts == next_counts
}

fn is_content_bearing(unit: &CacheUnit) -> bool {
    matches!(
        unit.kind,
        CacheUnitKind::VisibleText
            | CacheUnitKind::ToolDefinition
            | CacheUnitKind::ToolCall
            | CacheUnitKind::ToolResult
    )
}

type UnitFingerprint = (CacheUnitKind, String, String, ContextCollectionKind);

fn fingerprint(unit: &CacheUnit) -> UnitFingerprint {
    (
        unit.kind.clone(),
        unit.alignment_key.clone(),
        unit.content.clone(),
        unit.hierarchy.collection.clone(),
    )
}

/// 第一个分歧点之后，两条请求按原顺序连续对齐的最长相同单位段字节数（CC 证据）。
fn recovered_contiguous_bytes(prev: &CacheSequence, next: &CacheSequence, lcp: usize) -> usize {
    let mut best_bytes = 0usize;
    let mut best_units = 0usize;
    for next_start in lcp..next.units.len() {
        let mut run_bytes = 0usize;
        let mut run_units = 0usize;
        let mut prev_cursor = lcp;
        for unit in &next.units[next_start..] {
            let matched =
                prev_cursor < prev.units.len() && same_observable(unit, &prev.units[prev_cursor]);
            if matched {
                run_bytes += unit.content.len();
                run_units += 1;
                prev_cursor += 1;
                if run_bytes > best_bytes {
                    best_bytes = run_bytes;
                    best_units = run_units;
                }
            } else {
                run_bytes = 0;
                run_units = 0;
                prev_cursor = lcp;
            }
        }
    }
    let _ = best_units;
    best_bytes
}

fn observable_bytes(units: &[CacheUnit]) -> usize {
    units.iter().map(|unit| unit.content.len()).sum()
}

/// `requests[]` 的体积上限：超出时只保留增益最大的前若干条。
const MAX_PREFIX_REUSE_REQUESTS: usize = 500;

/// 单个请求的前缀上限与限制它的缺陷。
#[derive(Clone, Debug)]
pub(crate) struct RequestPotential {
    /// 限制该请求前缀的缺陷中，被阻断稳定字节的最大值。
    pub blocked_stable_bytes: usize,
    pub defect_ids: Vec<String>,
}

/// 从缺陷证据反推每个受影响请求被阻断的稳定字节。
///
/// 口径：取缺陷的 `blocked_stable_bytes`（X 之后本可复用却被阻断的稳定字节）。
/// 同一请求被多个缺陷限制时取最大值、**不做相加**——同一根因常在不同比较组里被重复识别，
/// 相加会重复计数；取最大值是保守估计。该量是结构代理，不代表真实 token 命中或金额收益。
pub(crate) fn request_potential_by_request(
    defects: &[ContextDefect],
) -> HashMap<String, RequestPotential> {
    let mut potentials: HashMap<String, RequestPotential> = HashMap::new();
    for defect in defects {
        for variant in &defect.mismatch.variants {
            for request_id in &variant.member_request_ids {
                let entry =
                    potentials
                        .entry(request_id.clone())
                        .or_insert_with(|| RequestPotential {
                            blocked_stable_bytes: 0,
                            defect_ids: Vec::new(),
                        });
                entry.blocked_stable_bytes =
                    entry.blocked_stable_bytes.max(defect.blocked_stable_bytes);
                entry.defect_ids.push(defect.id.clone());
            }
        }
    }
    for entry in potentials.values_mut() {
        entry.defect_ids.sort();
        entry.defect_ids.dedup();
    }
    potentials
}

/// 汇总结构复用率：Σ 会话内相邻请求对的公共前缀 / Σ 上下文总字节，
/// 并给出"消除已知结构问题后"的上界。
///
/// 口径说明（与 `docs/architectures/analyze/output.md` 保持一致）：
/// - 只覆盖会话内相邻请求对，会话首个请求没有前序、不参与统计；
/// - 无样本时返回 `None`，报告中不渲染该区块；
/// - 是结构代理，不代表真实 token 命中、延迟或金额收益。
///
/// 逐请求上限来自缺陷证据（见 [`request_potential_by_request`]）：
/// `上限 = 当前公共前缀 + 所涉缺陷被阻断稳定字节的最大值`，并以该请求上下文总长为上界。
/// 含义是"若这些结构问题都被消除，该请求可达的前缀"，属结构上界。
pub(crate) fn summarize_prefix_reuse(
    reports: &[SessionReport],
    potentials: &HashMap<String, RequestPotential>,
) -> Option<PrefixReuseSummary> {
    if reports.is_empty() {
        return None;
    }
    let mut by_session = Vec::with_capacity(reports.len());
    let mut requests: Vec<PrefixReuseRequest> = Vec::new();
    let mut request_pairs = 0usize;
    let mut lcp_bytes = 0usize;
    let mut potential_lcp_bytes = 0usize;
    let mut context_bytes = 0usize;
    let mut recoverable_stable_bytes = 0usize;
    let mut sessions_with_prefix_cut = 0usize;
    for report in reports {
        let mut session_pairs = 0usize;
        let mut session_lcp = 0usize;
        let mut session_potential = 0usize;
        let mut session_context = 0usize;
        for event in &report.events {
            let reusable = event.lcp_bytes;
            let potential = potentials
                .get(&event.next_request_id)
                .map(|entry| {
                    reusable
                        .saturating_add(entry.blocked_stable_bytes)
                        .min(event.next_total_bytes)
                        .max(reusable)
                })
                .unwrap_or(reusable);
            session_pairs += 1;
            session_lcp += reusable;
            session_potential += potential;
            session_context += event.next_total_bytes;
            if potential > reusable {
                requests.push(PrefixReuseRequest {
                    request_id: event.next_request_id.clone(),
                    session_key: report.session_key.clone(),
                    reusable_prefix_bytes: reusable,
                    potential_prefix_bytes: potential,
                    gain_bytes: potential - reusable,
                    blocking_defect_ids: potentials
                        .get(&event.next_request_id)
                        .map(|entry| entry.defect_ids.clone())
                        .unwrap_or_default(),
                });
            }
        }
        if report.total_prefix_cut_bytes > 0 {
            sessions_with_prefix_cut += 1;
        }
        recoverable_stable_bytes += report.total_stable_after_switch_bytes;
        request_pairs += session_pairs;
        lcp_bytes += session_lcp;
        potential_lcp_bytes += session_potential;
        context_bytes += session_context;
        by_session.push(PrefixReuseSession {
            session_key: report.session_key.clone(),
            request_pairs: session_pairs,
            lcp_bytes: session_lcp,
            context_bytes: session_context,
            ratio: reuse_ratio(session_lcp, session_context),
            potential_lcp_bytes: session_potential,
            gain_bytes: session_potential.saturating_sub(session_lcp),
        });
    }
    if request_pairs == 0 {
        return None;
    }
    requests.sort_by(|left, right| {
        right
            .gain_bytes
            .cmp(&left.gain_bytes)
            .then_with(|| left.request_id.cmp(&right.request_id))
    });
    requests.truncate(MAX_PREFIX_REUSE_REQUESTS);
    let gain_bytes = potential_lcp_bytes.saturating_sub(lcp_bytes);
    Some(PrefixReuseSummary {
        sessions: reports.len(),
        request_pairs,
        lcp_bytes,
        context_bytes,
        ratio: reuse_ratio(lcp_bytes, context_bytes),
        potential_lcp_bytes,
        potential_ratio: reuse_ratio(potential_lcp_bytes, context_bytes),
        gain_bytes,
        recoverable_stable_bytes,
        sessions_with_prefix_cut,
        by_session,
        requests,
        composition: PrefixReuseComposition {
            context_bytes,
            reusable_bytes: lcp_bytes,
            recoverable_bytes: gain_bytes,
            new_content_bytes: context_bytes.saturating_sub(potential_lcp_bytes),
        },
    })
}

fn reuse_ratio(lcp_bytes: usize, context_bytes: usize) -> f64 {
    if context_bytes == 0 {
        0.0
    } else {
        lcp_bytes as f64 / context_bytes as f64
    }
}
