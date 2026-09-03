//! 本文件将 Session 摘要、相邻请求时间线与历史变化位点安全渲染为静态 HTML。

use std::fmt::Write;

use actrail_kv_artifacts::{
    SessionAnalysis, SessionBoundaryContext, SessionDivergence, SessionHistorySiteKind,
    SessionPrefixMetrics, SessionTransitionOutcome,
};
use anyhow::Result;
use html_escape::encode_text;

pub(super) fn render_session_analysis(
    output: &mut String,
    analysis: &SessionAnalysis,
) -> Result<()> {
    output.push_str("<h2>Session 前缀延续</h2>");
    let transition_count: usize = analysis
        .timelines
        .iter()
        .map(|timeline| timeline.transitions.len())
        .sum();
    if analysis.session_record_count == 0 || transition_count == 0 {
        output.push_str("<p>未提供 Session ID 或没有可分析的时间线。</p>");
        return Ok(());
    }
    write!(
        output,
        "<p>Session 记录 {} 条；时间线 {} 条；相邻 transition {} 条；历史变化位点 {} 个。</p>",
        analysis.session_record_count,
        analysis.timelines.len(),
        transition_count,
        analysis.history_sites.len()
    )?;
    render_outcome_summary(output, analysis)?;
    output.push_str("<h3>Session 时间线</h3>");
    for timeline in &analysis.timelines {
        if timeline.transitions.is_empty() {
            continue;
        }
        write!(
            output,
            "<section class=\"timeline\"><h4>Session <code>{}</code></h4><table><thead><tr><th>相邻请求</th><th>分类</th><th>可观察前缀</th><th>首次分叉/原因</th><th>比较边界</th></tr></thead><tbody>",
            encode_text(&timeline.session_id)
        )?;
        for (index, transition) in timeline.transitions.iter().enumerate() {
            let previous_source = timeline.requests[index].source.as_deref();
            let current_source = timeline.requests[index + 1].source.as_deref();
            write!(
                output,
                "<tr><td><code>{}</code><br><span class=\"muted\">{}{}</span><br>→ <code>{}</code><br><span class=\"muted\">{}{}</span></td>",
                encode_text(&transition.previous_request_id),
                render_optional_text(&transition.previous_captured_at),
                render_source(previous_source),
                encode_text(&transition.current_request_id),
                render_optional_text(&transition.current_captured_at),
                render_source(current_source)
            )?;
            render_outcome_cells(output, &transition.outcome)?;
            output.push_str("<td>");
            if let Some(boundary) = &transition.boundary {
                render_boundary_context(output, boundary)?;
            } else {
                output.push('—');
            }
            output.push_str("</td>");
            output.push_str("</tr>");
        }
        output.push_str("</tbody></table></section>");
    }
    render_history_sites(output, analysis)?;
    Ok(())
}

fn render_outcome_summary(output: &mut String, analysis: &SessionAnalysis) -> Result<()> {
    let mut counts = [0usize; 7];
    for outcome in analysis.timelines.iter().flat_map(|timeline| {
        timeline
            .transitions
            .iter()
            .map(|transition| &transition.outcome)
    }) {
        counts[outcome_index(outcome)] += 1;
    }
    write!(
        output,
        "<p class=\"muted\">完全相同 {}；正常追加 {}；前缀截断 {}；历史变化 {}；不可比边界 {}；顺序歧义 {}；不可分析边界 {}。</p>",
        counts[0], counts[1], counts[2], counts[3], counts[4], counts[5], counts[6]
    )?;
    Ok(())
}

fn render_outcome_cells(output: &mut String, outcome: &SessionTransitionOutcome) -> Result<()> {
    match outcome {
        SessionTransitionOutcome::Identical { metrics } => {
            render_metrics_cells(output, "完全相同", "ok", metrics, None)
        }
        SessionTransitionOutcome::NormalAppend { metrics } => {
            render_metrics_cells(output, "正常追加", "ok", metrics, None)
        }
        SessionTransitionOutcome::PrefixTruncated {
            metrics,
            divergence,
        } => render_metrics_cells(output, "前缀截断", "warn", metrics, Some(divergence)),
        SessionTransitionOutcome::HistoryChanged {
            metrics,
            divergence,
        } => render_metrics_cells(output, "历史变化", "warn", metrics, Some(divergence)),
        SessionTransitionOutcome::IncomparableBoundary { reason } => {
            render_boundary_cells(output, "不可比边界", reason)
        }
        SessionTransitionOutcome::AmbiguousOrder { reason } => {
            render_boundary_cells(output, "顺序歧义", reason)
        }
        SessionTransitionOutcome::UnanalyzableBoundary { reason } => {
            render_boundary_cells(output, "不可分析边界", reason)
        }
    }
}

fn render_metrics_cells(
    output: &mut String,
    label: &str,
    class_name: &str,
    metrics: &SessionPrefixMetrics,
    divergence: Option<&SessionDivergence>,
) -> Result<()> {
    write!(
        output,
        "<td class=\"{}\">{}</td><td>上一请求 {} bytes；当前请求 {} bytes；保留 {} bytes（{:.1}%）；失效旧后缀 {} bytes</td><td>",
        class_name,
        label,
        metrics.previous_observable_bytes,
        metrics.current_observable_bytes,
        metrics.preserved_prefix_bytes,
        metrics.prefix_retention_ratio * 100.0,
        metrics.invalidated_previous_suffix_bytes
    )?;
    if let Some(divergence) = divergence {
        render_divergence(output, divergence)?;
    } else {
        output.push('—');
    }
    output.push_str("</td>");
    Ok(())
}

fn render_boundary_cells(output: &mut String, label: &str, reason: &str) -> Result<()> {
    write!(
        output,
        "<td>{}</td><td>不适用</td><td>{}</td>",
        label,
        encode_text(reason)
    )?;
    Ok(())
}

fn render_divergence(output: &mut String, divergence: &SessionDivergence) -> Result<()> {
    write!(
        output,
        "位置 <code>{}</code>",
        encode_text(&divergence.logical_position)
    )?;
    for (label, source, excerpt) in [
        (
            "上一请求",
            divergence.previous_source.as_ref(),
            divergence.previous_excerpt.as_ref(),
        ),
        (
            "当前请求",
            divergence.current_source.as_ref(),
            divergence.current_excerpt.as_ref(),
        ),
    ] {
        if source.is_some() || excerpt.is_some() {
            write!(output, "<br>{label}：")?;
        }
        if let Some(source) = source {
            write!(output, "<code>{}</code> ", encode_text(&source.json_path))?;
        }
        if let Some(excerpt) = excerpt {
            write!(output, "<code>{}</code>", encode_text(excerpt))?;
        }
    }
    Ok(())
}

fn render_history_sites(output: &mut String, analysis: &SessionAnalysis) -> Result<()> {
    output.push_str("<h3>历史变化位点</h3>");
    if analysis.history_sites.is_empty() {
        output.push_str("<p>时间线中没有观察到既有历史变化。</p>");
        return Ok(());
    }
    for site in &analysis.history_sites {
        write!(
            output,
            "<section class=\"site history\"><h4>{} · <code>{}</code></h4><p>逻辑位置 <code>{}</code>；发生 {} 次；影响 {} 个 Session；失效旧后缀合计 {} bytes，单次 {}–{} bytes。</p><p>比较边界：",
            history_kind_label(&site.kind),
            encode_text(&site.id),
            encode_text(&site.logical_position),
            site.occurrence_count,
            site.affected_session_count,
            site.invalidated_previous_suffix_bytes_total,
            site.invalidated_previous_suffix_bytes_min,
            site.invalidated_previous_suffix_bytes_max
        )?;
        render_boundary_context(output, &site.boundary)?;
        output.push_str("</p>");
        output.push_str("<p>Transitions：");
        for (index, transition_id) in site.transition_ids.iter().enumerate() {
            if index > 0 {
                output.push_str(", ");
            }
            write!(output, "<code>{}</code>", encode_text(transition_id))?;
        }
        output.push_str("</p>");
        if let Some(insight) = &site.insight {
            write!(
                output,
                "<p><strong>{}</strong>",
                encode_text(&insight.summary)
            )?;
            if let Some(detail) = &insight.detail {
                write!(output, "：{}", encode_text(detail))?;
            }
            output.push_str("</p>");
        }
        output.push_str("</section>");
    }
    Ok(())
}

fn render_boundary_context(output: &mut String, boundary: &SessionBoundaryContext) -> Result<()> {
    write!(
        output,
        "<code>endpoint={} model={} schema={} dialect={} adapter={}</code>",
        encode_text(&boundary.endpoint_key),
        encode_text(&boundary.model),
        encode_text(&boundary.context_schema_key),
        encode_text(&boundary.dialect),
        encode_text(&boundary.adapter_revision)
    )?;
    for (label, value) in [
        ("agent", boundary.agent_key.as_deref()),
        ("deployment", boundary.model_deployment_key.as_deref()),
        ("namespace", boundary.kv_namespace.as_deref()),
    ] {
        if let Some(value) = value {
            write!(output, "<br><code>{label}={}</code>", encode_text(value))?;
        }
    }
    Ok(())
}

fn history_kind_label(kind: &SessionHistorySiteKind) -> &'static str {
    match kind {
        SessionHistorySiteKind::HistoryChanged => "历史变化",
        SessionHistorySiteKind::PrefixTruncated => "前缀截断",
    }
}

fn outcome_index(outcome: &SessionTransitionOutcome) -> usize {
    match outcome {
        SessionTransitionOutcome::Identical { .. } => 0,
        SessionTransitionOutcome::NormalAppend { .. } => 1,
        SessionTransitionOutcome::PrefixTruncated { .. } => 2,
        SessionTransitionOutcome::HistoryChanged { .. } => 3,
        SessionTransitionOutcome::IncomparableBoundary { .. } => 4,
        SessionTransitionOutcome::AmbiguousOrder { .. } => 5,
        SessionTransitionOutcome::UnanalyzableBoundary { .. } => 6,
    }
}

fn render_optional_text(value: &Option<String>) -> std::borrow::Cow<'_, str> {
    value
        .as_ref()
        .map(encode_text)
        .unwrap_or_else(|| "未提供时间".into())
}

fn render_source(source: Option<&str>) -> String {
    source
        .map(|value| format!(" · source={}", encode_text(value)))
        .unwrap_or_default()
}
