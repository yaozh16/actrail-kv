//! 本文件将统一 P1/X/P2 缺陷结果渲染为无脚本、全转义的静态 HTML。

use std::{collections::BTreeMap, fmt::Write};

use actrail_kv_artifacts::{
    AnalysisResult, ContextDefect, DefectFactKind, MismatchPattern, SessionEventType,
};
use anyhow::{anyhow, Result};
use html_escape::encode_text;

pub fn render_html(result: &AnalysisResult) -> Result<String> {
    let defects: BTreeMap<_, _> = result
        .defects
        .iter()
        .map(|defect| (defect.id.as_str(), defect))
        .collect();
    let mut output = String::with_capacity(16_384);
    output.push_str("<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\">");
    output.push_str("<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">");
    output.push_str("<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'\">");
    output.push_str("<title>Actrail KV 结构诊断</title><style>");
    output.push_str("body{font:15px/1.55 system-ui,sans-serif;margin:2rem;max-width:1100px;color:#17202a}table{border-collapse:collapse;width:100%}th,td{border:1px solid #d5d8dc;padding:.55rem;text-align:left;vertical-align:top}th{background:#f4f6f7}.defect{border:1px solid #ccd1d1;border-radius:8px;padding:1rem;margin:1rem 0}.muted{color:#626567}code{white-space:pre-wrap;overflow-wrap:anywhere}.sequence{font-size:1.05rem}.variant{margin:.5rem 0;padding:.5rem;background:#f8f9f9}.pt-summary{display:flex;gap:1rem;flex-wrap:wrap;margin:.4rem 0 1rem}.pt-metric{background:#f4f6f7;border:1px solid #e0e3e5;border-radius:6px;padding:.4rem .8rem}.pt-metric b{display:block;font-size:1.2rem}.pt-table{border-collapse:collapse;width:100%;margin:.6rem 0 1.2rem}.pt-table th,.pt-table td{border:1px solid #d5d8dc;padding:.4rem;vertical-align:top;font-size:13px}.pt-table th{background:#f4f6f7}</style></head><body>");
    output.push_str("<h1>LLM KV 缓存结构诊断</h1>");
    write!(
        output,
        "<p>输入 {} 条，分析 {} 条，跳过 {} 条；发现 {} 个上下文结构缺陷。</p>",
        result.run.input_records,
        result.run.analyzed_records,
        result.run.skipped_records.len(),
        result.defects.len()
    )?;
    output.push_str("<p class=\"muted\">本报告基于可观察 HTTP payload 的结构代理，不代表真实 Token、KV 命中率或成本收益。</p>");
    output.push_str("<h2>Top K</h2><table><thead><tr><th>排名</th><th>缺陷</th><th>得分</th><th>受影响请求</th><th>被阻断稳定字节</th></tr></thead><tbody>");
    for (index, id) in result.top_k.iter().enumerate() {
        let defect = defects
            .get(id.as_str())
            .ok_or_else(|| anyhow!("top_k references unknown defect {id}"))?;
        write!(
            output,
            "<tr><td>{}</td><td><code>{}</code></td><td>{:.2}</td><td>{}</td><td>{}</td></tr>",
            index + 1,
            encode_text(id),
            defect.score.score,
            defect.affected_count,
            defect.blocked_stable_bytes
        )?;
    }
    output.push_str("</tbody></table><h2>缺陷详情</h2>");
    for defect in &result.defects {
        render_defect(&mut output, defect)?;
    }
    if !result.session_reports.is_empty() {
        output.push_str("<h2>会话 Prefix-Switch 证据</h2>");
        for report in &result.session_reports {
            render_session_report(&mut output, report)?;
        }
    }
    if !result.cache_metrics.is_empty() {
        render_cache_metrics(&mut output, result)?;
    }
    output.push_str("</body></html>");
    Ok(output)
}

fn render_session_report(
    output: &mut String,
    report: &actrail_kv_artifacts::SessionReport,
) -> Result<()> {
    write!(
        output,
        "<section class=\"defect\"><h3>session={} endpoint={} model={}</h3><p>请求数 {}；append {}，fork {}，reorder {}，reset {}；切换后需重算 {} bytes，可恢复稳定 {} bytes；前缀收缩共 {} bytes（平均 {}）。</p><table><thead><tr><th>类型</th><th>前一请求</th><th>后一请求</th><th>公共前缀</th><th>前缀收缩</th><th>重算 bytes</th><th>可恢复稳定 bytes</th><th>块 run</th></tr></thead><tbody>",
        encode_text(&report.session_key),
        encode_text(&report.endpoint_key),
        encode_text(&report.model),
        report.request_count,
        report.append_count,
        report.fork_count,
        report.reorder_count,
        report.reset_count,
        report.total_recomputed_bytes,
        report.total_stable_after_switch_bytes,
        report.total_prefix_cut_bytes,
        report.avg_prefix_cut_bytes
    )?;
    for event in &report.events {
        write!(
            output,
            "<tr><td>{}</td><td><code>{}</code></td><td><code>{}</code></td><td>{} units / {} bytes</td><td>{}</td><td>{}</td><td>{}</td><td><code>{}</code></td></tr>",
            event_label(&event.event_type),
            encode_text(&event.prev_request_id),
            encode_text(&event.next_request_id),
            event.lcp_units,
            event.lcp_bytes,
            event.prefix_cut_bytes,
            event.recomputed_bytes,
            event.stable_after_switch_bytes,
            encode_text(&event.next_block_runs)
        )?;
    }
    output.push_str("</tbody></table></section>");
    Ok(())
}

fn event_label(event: &SessionEventType) -> &'static str {
    match event {
        SessionEventType::Append => "append",
        SessionEventType::Fork => "fork",
        SessionEventType::Reorder => "reorder",
        SessionEventType::Reset => "reset",
    }
}

fn render_cache_metrics(output: &mut String, result: &AnalysisResult) -> Result<()> {
    output.push_str(
        "<h2>KV 缓存命中率</h2>\
         <p class=\"muted\">命中率 = cached_tokens / prompt_tokens（真实 usage）；\
         无 usage 时按与会话内上一条请求的上下文公共前缀估算，并在“口径”列标注。</p>",
    );
    let reported = result
        .cache_metrics
        .iter()
        .filter(|metric| metric.basis == actrail_kv_artifacts::CacheMetricBasis::Reported)
        .count();
    let estimated = result.cache_metrics.len().saturating_sub(reported);
    let average = if result.cache_metrics.is_empty() {
        0.0
    } else {
        result
            .cache_metrics
            .iter()
            .map(|metric| metric.hit_rate)
            .sum::<f64>()
            / result.cache_metrics.len() as f64
    };
    write!(
        output,
        "<div class=\"pt-summary\"><div class=\"pt-metric\"><b>{}</b>请求</div>\
         <div class=\"pt-metric\"><b>{:.1}%</b>平均命中率</div>\
         <div class=\"pt-metric\"><b>{}</b>真实 usage</div>\
         <div class=\"pt-metric\"><b>{}</b>估算</div></div>",
        result.cache_metrics.len(),
        average * 100.0,
        reported,
        estimated
    )?;
    output.push_str(
        "<table class=\"pt-table\"><thead><tr><th>会话</th><th>序号</th><th>请求</th>\
         <th>prompt</th><th>hit(cached)</th><th>miss</th><th>output</th>\
         <th>TTFT</th><th>总耗时</th><th>命中率</th><th>Δ</th><th>口径</th><th>明细</th></tr></thead><tbody>",
    );
    let mut current_session: Option<Option<&str>> = None;
    for metric in &result.cache_metrics {
        let session = metric.session_key.as_deref();
        if current_session != Some(session) {
            current_session = Some(session);
            let label = match session {
                Some(value) => {
                    let head = &value[..value.len().min(8)];
                    let tail_start = value.len().saturating_sub(12);
                    format!("{}…{}", head, &value[tail_start..])
                }
                None => "（无 session，按输入顺序）".to_string(),
            };
            write!(
                output,
                "<tr><td colspan=\"13\" style=\"background:#f8f9f9\"><strong>会话 {}</strong></td></tr>",
                encode_text(&label)
            )?;
        }
        let tokens = |value: Option<u32>| {
            value
                .map(|tokens| tokens.to_string())
                .unwrap_or_else(|| "—".to_string())
        };
        // 时延是上游观测值，未上报时显示 —，不显示 0。
        let latency = |value: Option<u64>| {
            value
                .map(|ms| format!("{ms} ms"))
                .unwrap_or_else(|| "—".to_string())
        };
        let delta = metric
            .hit_rate_delta
            .map(|delta| {
                format!(
                    "{}{:.1}pp",
                    if delta >= 0.0 { "+" } else { "" },
                    delta * 100.0
                )
            })
            .unwrap_or_else(|| "—".to_string());
        let detail = match metric.basis {
            actrail_kv_artifacts::CacheMetricBasis::Reported => "—".to_string(),
            actrail_kv_artifacts::CacheMetricBasis::Estimated => format!(
                "LCP {}B / payload {}B",
                metric.estimated_lcp_bytes.unwrap_or(0),
                metric.payload_bytes
            ),
        };
        write!(
            output,
            "<tr><td></td><td>{}</td><td><code>{}</code></td><td>{}</td><td>{}</td>\
             <td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{:.1}%</td><td>{}</td>\
             <td>{}</td><td>{}</td></tr>",
            metric.sequence,
            encode_text(&metric.request_id[..metric.request_id.len().min(12)]),
            tokens(metric.prompt_tokens),
            tokens(metric.cached_tokens),
            tokens(metric.miss_tokens),
            tokens(metric.output_tokens),
            latency(metric.ttft_ms),
            latency(metric.total_ms),
            metric.hit_rate * 100.0,
            encode_text(&delta),
            match metric.basis {
                actrail_kv_artifacts::CacheMetricBasis::Reported => "真实",
                actrail_kv_artifacts::CacheMetricBasis::Estimated => "估算",
            },
            encode_text(&detail)
        )?;
    }
    output.push_str("</tbody></table>");
    Ok(())
}

fn render_defect(output: &mut String, defect: &ContextDefect) -> Result<()> {
    write!(
        output,
        "<section class=\"defect\"><h3>{}</h3><p><strong>比较组：</strong><code>window={} endpoint={} model={} schema={}</code></p>",
        encode_text(&defect.id),
        encode_text(&defect.comparison_group.time_window_key),
        encode_text(&defect.comparison_group.endpoint_key),
        encode_text(&defect.comparison_group.model),
        encode_text(&defect.comparison_group.context_schema_key)
    )?;
    if let Some(agent) = &defect.comparison_group.agent_key {
        write!(
            output,
            "<p><strong>Agent：</strong><code>{}</code></p>",
            encode_text(agent)
        )?;
    }
    if let Some(deployment) = &defect.comparison_group.model_deployment_key {
        write!(
            output,
            "<p><strong>模型部署：</strong><code>{}</code></p>",
            encode_text(deployment)
        )?;
    }
    if let Some(namespace) = &defect.comparison_group.kv_namespace {
        write!(
            output,
            "<p><strong>KV namespace：</strong><code>{}</code></p>",
            encode_text(namespace)
        )?;
    }
    write!(
        output,
        "<p class=\"sequence\"><strong>P1</strong> 公共前缀 {} bytes → <strong>X</strong> {} → <strong>P2</strong> 恢复稳定 {} bytes（支持 {} 条）</p>",
        defect.actual_prefix_bytes,
        pattern_label(&defect.mismatch.pattern),
        defect.recovered_stable.utf8_bytes,
        defect.recovered_stable.support_count
    )?;
    output.push_str("<h4>X 变体</h4>");
    for variant in &defect.mismatch.variants {
        let evidence = &variant.representative;
        write!(
            output,
            "<div class=\"variant\"><strong>{} 条请求</strong>；代表请求 <code>{}</code>；{} bytes",
            variant.member_request_ids.len(),
            encode_text(&evidence.request_id),
            evidence.utf8_bytes
        )?;
        if !evidence.sources.is_empty() {
            output.push_str("；位置 ");
            for (index, source) in evidence.sources.iter().enumerate() {
                if index > 0 {
                    output.push_str(", ");
                }
                write!(output, "<code>{}</code>", encode_text(&source.json_path))?;
            }
        }
        match &evidence.excerpt {
            Some(excerpt) => write!(output, "<br><code>{}</code>", encode_text(excerpt))?,
            None => output.push_str("<br><em>该变体在此区域缺失内容</em>"),
        }
        output.push_str("</div>");
    }
    if !defect.mismatch.facts.is_empty() {
        output.push_str("<h4>已确认事实</h4><ul>");
        for fact in &defect.mismatch.facts {
            write!(output, "<li>{}", fact_label(&fact.kind))?;
            if let Some(detail) = &fact.detail {
                write!(output, "：{}", encode_text(detail))?;
            }
            output.push_str("</li>");
        }
        output.push_str("</ul>");
    }
    write!(
        output,
        "<h4>P2 稳定证据</h4><p><code>{}</code></p><p>实际前缀 {} bytes；潜在前缀 {} bytes；阻断 {} bytes；可比较 {} 条；影响 {} 条；置信度 {:.3}。</p>",
        encode_text(&defect.recovered_stable.excerpt),
        defect.actual_prefix_bytes,
        defect.potential_prefix_bytes,
        defect.blocked_stable_bytes,
        defect.comparable_count,
        defect.affected_count,
        defect.confidence
    )?;
    if !defect.insights.is_empty() {
        output.push_str("<h4>优化启示</h4><ul>");
        for insight in &defect.insights {
            write!(
                output,
                "<li><strong>{}</strong>",
                encode_text(&insight.summary)
            )?;
            if let Some(detail) = &insight.detail {
                write!(output, "：{}", encode_text(detail))?;
            }
            output.push_str("</li>");
        }
        output.push_str("</ul>");
    }
    output.push_str("</section>");
    Ok(())
}

fn pattern_label(pattern: &MismatchPattern) -> &'static str {
    match pattern {
        MismatchPattern::ValueMismatch => "同一位置内容不同",
        MismatchPattern::InsertionDeletion => "插入或缺失",
        MismatchPattern::Reorder => "移动或重排",
        MismatchPattern::Mixed => "混合变体",
    }
}

fn fact_label(kind: &DefectFactKind) -> &'static str {
    match kind {
        DefectFactKind::ContentVariation => "内容持续变化",
        DefectFactKind::FixedVariants => "少数固定版本",
        DefectFactKind::StructuredDataEquivalent => "结构化数据等价",
        DefectFactKind::InsertionDeletion => "存在插入或缺失",
        DefectFactKind::Reorder => "存在可证明的移动或重排",
        DefectFactKind::Scope => "模型可见层级范围",
    }
}
