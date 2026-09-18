//! 本文件将统一 P1/X/P2 缺陷结果渲染为无脚本、全转义的静态 HTML。

use std::{collections::BTreeMap, fmt::Write};

use actrail_kv_artifacts::{
    AnalysisResult, ContextDefect, DefectFactKind, MismatchPattern, PrefixNodeKind,
    SessionEventType,
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
    output.push_str("<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'\">");
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
    if result
        .templates
        .iter()
        .any(|template| template.prefix_view.is_some())
    {
        render_prefix_tree(&mut output, result)?;
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
        "<section class=\"defect\" id=\"defect-{}\"><h3>{}</h3><p><strong>比较组：</strong><code>window={} endpoint={} model={} schema={}</code></p>",
        encode_text(&defect.id),
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

const PREFIX_TREE_CSS: &str = "\
.pt-legend{display:grid;gap:.3rem;margin:.6rem 0 1rem;padding:.7rem .9rem;background:#f8f9f9;border:1px solid #e0e3e5;border-radius:6px;font-size:13px}\
.pt-legend b{margin-right:.35rem}\
.pt-badge{display:inline-block;min-width:1.25rem;text-align:center;border-radius:3px;padding:.05rem .3rem;font-size:11px;font-weight:700;color:#fff;line-height:1.3}\
.pt-badge-p{background:#2c6fb0}\
.pt-badge-x{background:#b9770e}\
.pt-badge-d{background:#c0392b}\
.pt-toc{border-collapse:collapse;width:100%;font-size:13px;margin:.4rem 0 1rem}\
.pt-toc th,.pt-toc td{border:1px solid #d5d8dc;padding:.4rem .5rem;text-align:left}\
.pt-toc th{background:#f4f6f7}\
.pt-section{margin:1.4rem 0;padding:1rem 1.1rem;border:1px solid #d5d8dc;border-radius:8px;background:#fdfdfd}\
.pt-head h3{margin:0 0 .2rem;font-size:16px}\
.pt-sub{margin:0;color:#626567;font-size:13px}\
.pt-stats{display:flex;flex-wrap:wrap;gap:.5rem;margin:.8rem 0 .4rem}\
.pt-stat{min-width:6rem;padding:.45rem .7rem;background:#f4f6f7;border:1px solid #e0e3e5;border-radius:6px}\
.pt-stat-k{display:block;font-size:11px;color:#626567}\
.pt-stat-v{display:block;font-size:17px;font-weight:600}\
.pt-h{margin:1.2rem 0 .35rem;font-size:14px;color:#1a5276;border-left:3px solid #2980b9;padding-left:.5rem}\
.pt-note{margin:.35rem 0 .6rem;color:#626567;font-size:12.5px}\
.pt-bar{display:flex;height:14px;border:1px solid #d5d8dc;border-radius:4px;overflow:hidden;background:#fff}\
.pt-bar i{display:block;height:100%}\
.pt-nodes{list-style:none;margin:.4rem 0 0;padding:0;display:flex;flex-direction:column;gap:.3rem}\
.pt-node{border:1px solid #d5d8dc;border-radius:6px;background:#fff}\
.pt-node.is-p{border-left:4px solid #2c6fb0}\
.pt-node.is-x{border-left:4px solid #b9770e;background:#fffdf7}\
.pt-node.is-defect{border-left-color:#c0392b;background:#fdf4f3}\
.pt-node summary{display:flex;align-items:center;gap:.5rem;padding:.42rem .6rem;cursor:pointer;font-size:13px}\
.pt-idx{flex:0 0 auto;color:#95a5a6;font-size:11px;min-width:1.9rem}\
.pt-excerpt{flex:1 1 auto;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-family:ui-monospace,Menlo,Consolas,monospace}\
.pt-size{flex:0 0 auto;color:#626567;font-size:11.5px}\
.pt-sup{flex:0 0 auto;display:flex;align-items:center;gap:.3rem;color:#626567;font-size:11.5px}\
.pt-sup-bar{display:inline-block;width:36px;height:5px;background:#e5e8ea;border-radius:3px;overflow:hidden}\
.pt-sup-bar i{display:block;height:100%;background:#2c6fb0}\
.pt-node.is-x .pt-sup-bar i{background:#b9770e}\
.pt-node.is-defect .pt-sup-bar i{background:#c0392b}\
.pt-warn{flex:0 0 auto;color:#c0392b;font-size:11.5px;font-weight:600}\
.pt-body{padding:.1rem .6rem .55rem;border-top:1px dashed #e0e3e5;background:#fff}\
.pt-fields{width:100%;border-collapse:collapse;font-size:12.5px}\
.pt-fields th{width:4.6rem;padding:.32rem .5rem .32rem 0;text-align:left;vertical-align:top;color:#626567;font-weight:500;background:none;border:none}\
.pt-fields td{padding:.32rem 0;border:none;vertical-align:top;overflow-wrap:anywhere}\
.pt-fields tr+tr th,.pt-fields tr+tr td{border-top:1px solid #f1f3f4}\
.pt-tag{display:inline-block;margin-left:.35rem;padding:0 .3rem;background:#eef2f5;border-radius:3px;color:#5d6d7e;font-size:11px}\
.pt-sec{margin:0;padding:.4rem .5rem;background:#f8f9f9;border:1px solid #eceff1;border-radius:4px;white-space:pre-wrap;overflow-wrap:anywhere;font-size:12px;max-height:13rem;overflow:auto}\
.pt-varlist{list-style:none;margin:0;padding:0;display:flex;flex-direction:column;gap:.15rem}\
.pt-var{display:flex;gap:.45rem;align-items:baseline}\
.pt-var code{flex:1 1 auto;min-width:0;overflow-wrap:anywhere}\
.pt-var em{flex:0 0 auto;font-style:normal;color:#626567;font-size:11.5px}\
.pt-var.is-missing code{color:#a04000}\
.pt-members{color:#626567;font-size:12px}\
";

fn render_prefix_tree(output: &mut String, result: &AnalysisResult) -> Result<()> {
    let templates_with_view: Vec<_> = result
        .templates
        .iter()
        .filter_map(|template| template.prefix_view.as_ref().map(|view| (template, view)))
        .collect();
    if templates_with_view.is_empty() {
        return Ok(());
    }

    output.push_str("<section id=\"prefix-tree\"><h2>模板前缀树</h2><style>");
    output.push_str(PREFIX_TREE_CSS);
    output.push_str("</style>");
    output.push_str("<div class=\"pt-legend\">");
    output.push_str(
        "<div><b class=\"pt-badge pt-badge-p\">P</b>稳定片段：区间内所有成员内容一致，可被后续请求复用。</div>",
    );
    output.push_str(
        "<div><b class=\"pt-badge pt-badge-x\">X</b>变体片段：成员在这里分叉，展开可见各变体及出现次数。</div>",
    );
    output.push_str(
        "<div><b class=\"pt-badge pt-badge-d\">X</b>红色 X：该分叉阻断了其后的稳定前缀，已被判为结构缺陷。</div>",
    );
    output.push_str(
        "<div>支持度 <code>3/4</code>：4 个成员请求中有 3 个包含该片段，右侧细条为占比。</div>",
    );
    output.push_str(
        "<div>每个模板先给分布与衰减概览，再按顺序列出结构片段；片段行只显示类型、摘要、字节数与支持度，\
         点开任意一行查看该片段的来源、位置、变体与成员明细。</div>",
    );
    output.push_str("</div>");

    if templates_with_view.len() > 1 {
        output.push_str(
            "<table class=\"pt-toc\"><thead><tr><th>模板</th><th>成员</th>\
             <th>稳定字节</th><th>变体片段</th><th>缺陷片段</th></tr></thead><tbody>",
        );
        for (template, view) in &templates_with_view {
            let stable_bytes: usize = view
                .nodes
                .iter()
                .filter(|node| node.kind == PrefixNodeKind::Stable)
                .map(|node| node.utf8_bytes)
                .sum();
            let dynamic_count = view
                .nodes
                .iter()
                .filter(|node| node.kind == PrefixNodeKind::Dynamic)
                .count();
            let defect_count = view
                .nodes
                .iter()
                .filter(|node| node.kind == PrefixNodeKind::Dynamic && !node.defect_ids.is_empty())
                .count();
            write!(
                output,
                "<tr><td><a href=\"#pt-{id}\"><code>{short}…</code></a></td><td>{members}</td>\
                 <td>{stable_bytes} B</td><td>{dynamic_count}</td><td>{defect_count}</td></tr>",
                id = encode_text(&template.id),
                short = encode_text(&template.id.chars().take(16).collect::<String>()),
                members = template.member_request_ids.len(),
            )?;
        }
        output.push_str("</tbody></table>");
    }

    for (template, view) in &templates_with_view {
        render_template_prefix_section(output, template, view)?;
    }
    output.push_str("</section>");
    Ok(())
}

fn render_template_prefix_section(
    output: &mut String,
    template: &actrail_kv_artifacts::RequestTemplate,
    view: &actrail_kv_artifacts::TemplatePrefixView,
) -> Result<()> {
    let member_count = template.member_request_ids.len();
    let stable_nodes: Vec<_> = view
        .nodes
        .iter()
        .filter(|node| node.kind == PrefixNodeKind::Stable)
        .collect();
    let dynamic_nodes: Vec<_> = view
        .nodes
        .iter()
        .filter(|node| node.kind == PrefixNodeKind::Dynamic)
        .collect();
    let stable_bytes: usize = stable_nodes.iter().map(|node| node.utf8_bytes).sum();
    let dynamic_bytes: usize = dynamic_nodes.iter().map(|node| node.utf8_bytes).sum();
    let variant_total: usize = dynamic_nodes
        .iter()
        .map(|node| node.variants.as_ref().map(Vec::len).unwrap_or(0))
        .sum();
    let missing_total: usize = dynamic_nodes
        .iter()
        .flat_map(|node| node.variants.iter().flatten())
        .filter(|variant| variant.excerpt.starts_with("（此位置缺失"))
        .count();
    let defect_nodes = dynamic_nodes
        .iter()
        .filter(|node| !node.defect_ids.is_empty())
        .count();
    let by_id: std::collections::HashMap<&str, &actrail_kv_artifacts::PrefixNode> = view
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    let mut chain = Vec::new();
    if let Some(mut current) = by_id.get("seq-0").copied() {
        loop {
            chain.push(current);
            match current
                .sequence_child
                .as_deref()
                .and_then(|id| by_id.get(id).copied())
            {
                Some(next) => current = next,
                None => break,
            }
        }
    }
    let first_dynamic = chain
        .iter()
        .position(|node| node.kind == PrefixNodeKind::Dynamic);
    let stable_before_first_x: usize = match first_dynamic {
        Some(end) => chain[..end].iter().map(|node| node.utf8_bytes).sum(),
        None => stable_bytes,
    };

    write!(
        output,
        "<div class=\"pt-section\" id=\"pt-{anchor}\"><div class=\"pt-head\"><h3>模板 {short}…</h3>\
         <p class=\"pt-sub\">完整 ID <code>{id}</code></p>\
         <p class=\"pt-sub\">{members} 个成员请求 · 结构片段 {nodes} 个（P {stable_nodes} / X {dynamic_nodes}）</p></div>",
        anchor = encode_text(&template.id),
        short = encode_text(&template.id.chars().take(16).collect::<String>()),
        id = encode_text(&template.id),
        members = member_count,
        nodes = view.nodes.len(),
        stable_nodes = stable_nodes.len(),
        dynamic_nodes = dynamic_nodes.len()
    )?;

    output.push_str("<div class=\"pt-stats\">");
    for (label, value) in [
        ("稳定字节", format!("{stable_bytes} B")),
        ("变体区字节", format!("{dynamic_bytes} B")),
        ("变体总数", variant_total.to_string()),
        ("缺失变体", missing_total.to_string()),
        ("缺陷片段", defect_nodes.to_string()),
    ] {
        write!(
            output,
            "<div class=\"pt-stat\"><span class=\"pt-stat-k\">{label}</span><span class=\"pt-stat-v\">{value}</span></div>"
        )?;
    }
    output.push_str("</div>");

    // D7-1 前缀复用分布
    let total_bytes = stable_bytes.saturating_add(dynamic_bytes).max(1);
    let stable_pct = stable_bytes as f64 * 100.0 / total_bytes as f64;
    let dynamic_pct = dynamic_bytes as f64 * 100.0 / total_bytes as f64;
    output.push_str("<h4 class=\"pt-h\">复用分布</h4>");
    write!(
        output,
        "<div class=\"pt-bar\">\
         <i style=\"width:{stable_pct:.1}%;background:#2c6fb0\"></i>\
         <i style=\"width:{dynamic_pct:.1}%;background:#e67e22\"></i></div>\
         <p class=\"pt-note\">稳定前缀 <b>{stable_bytes} B</b>（{stable_pct:.1}%）· \
         变体/缺失区 <b>{dynamic_bytes} B</b>（{dynamic_pct:.1}%）· \
         首个分叉点之前累计稳定前缀 <b>{stable_before_first_x} B</b>。</p>",
        stable_pct = stable_pct,
        dynamic_pct = dynamic_pct,
        stable_bytes = stable_bytes,
        dynamic_bytes = dynamic_bytes,
        stable_before_first_x = stable_before_first_x
    )?;
    if let Some(first_x) = first_dynamic {
        let node = chain[first_x];
        write!(
            output,
            "<p class=\"pt-note\">首个分叉点在第 {} 段「{}」，{}/{} 个成员包含该片段（{:.0}%）。</p>",
            first_x + 1,
            encode_text(&excerpt_label(&node.excerpt, 48)),
            node.support_count,
            member_count,
            node.support_ratio * 100.0
        )?;
    }

    // D7-2 成员衰减曲线（语义链节点序 → 存活成员数）
    if chain.len() > 1 {
        output.push_str("<h4 class=\"pt-h\">成员衰减</h4>");
        output.push_str("<svg viewBox=\"0 0 640 190\" style=\"width:100%;max-height:220px;background:#fff;border:1px solid #e0e3e5;border-radius:4px\">");
        for (ratio, label) in [(1.0_f64, "100%"), (0.5, "50%"), (0.0, "0%")] {
            let grid_y = 150 - (ratio * 120.0).round() as i32;
            write!(
                output,
                "<line x1=\"24\" y1=\"{grid_y}\" x2=\"616\" y2=\"{grid_y}\" stroke=\"#eceff1\"/>\
                 <text x=\"20\" y=\"{label_y}\" text-anchor=\"end\" font-size=\"9\" fill=\"#95a5a6\">{label}</text>",
                label_y = grid_y + 3
            )?;
        }
        let mut points = String::new();
        let n = chain.len();
        for (index, node) in chain.iter().enumerate() {
            let x = if n > 1 {
                24 + index * 592 / (n - 1)
            } else {
                24
            };
            let ratio = node.support_count as f64 / member_count.max(1) as f64;
            let y = 150 - (ratio * 120.0).round() as i32;
            points.push_str(&format!("{x},{y} "));
            write!(
                output,
                "<circle cx=\"{x}\" cy=\"{y}\" r=\"2.5\" fill=\"{}\"/>",
                if node.kind == PrefixNodeKind::Dynamic {
                    "#e67e22"
                } else {
                    "#2980b9"
                }
            )?;
        }
        write!(
            output,
            "<polyline points=\"{points}\" fill=\"none\" stroke=\"#5d6d7e\" stroke-width=\"1.5\"/>"
        )?;
        let label_step = (n / 10).max(1);
        for (index, node) in chain.iter().enumerate() {
            if index % label_step != 0 && index != n - 1 {
                continue;
            }
            let x = if n > 1 {
                24 + index * 592 / (n - 1)
            } else {
                24
            };
            let y = 150
                - (node.support_count as f64 / member_count.max(1) as f64 * 120.0).round() as i32;
            write!(
                output,
                "<line x1=\"{x}\" y1=\"{y}\" x2=\"{x}\" y2=\"150\" stroke=\"#d5d8dc\" stroke-dasharray=\"2 3\"/>\
                 <text x=\"{x}\" y=\"172\" text-anchor=\"middle\" font-size=\"9\" fill=\"#626567\">{index}</text>",
                index = index
            )?;
        }
        output.push_str("</svg>");
        output.push_str(
            "<p class=\"pt-note\">横轴为结构片段序号（P 蓝、X 橙），纵轴为仍包含该片段的成员占比；\
             曲线每次下坠都代表一批成员从该片段开始分叉。</p>",
        );
    }

    output.push_str("<h4 class=\"pt-h\">结构骨架</h4>");
    output.push_str(
        "<p class=\"pt-note\">按顺序列出每个片段，行内只保留类型、摘要、字节数与支持度；\
         点开任意一行查看该片段的来源、字节区间、变体与成员明细。</p>",
    );
    output.push_str("<ol class=\"pt-nodes\">");
    for (index, node) in chain.iter().enumerate() {
        render_prefix_node(output, template, node, index, member_count)?;
    }
    output.push_str("</ol>");
    if view.truncated {
        output.push_str("<p class=\"pt-note\">该模板节点数超过上限，视图已截断。</p>");
    }
    output.push_str("</div>");
    Ok(())
}

/// 单个结构片段：行内只保留可扫读的类型、摘要、字节数与支持度，其余字段在展开体里分行展示。
fn render_prefix_node(
    output: &mut String,
    template: &actrail_kv_artifacts::RequestTemplate,
    node: &actrail_kv_artifacts::PrefixNode,
    index: usize,
    member_count: usize,
) -> Result<()> {
    let is_dynamic = node.kind == PrefixNodeKind::Dynamic;
    let is_defect = !node.defect_ids.is_empty();
    let mut row_class = if is_dynamic {
        "pt-node is-x".to_string()
    } else {
        "pt-node is-p".to_string()
    };
    if is_defect {
        row_class.push_str(" is-defect");
    }
    let (badge_class, kind_label) = if is_dynamic {
        ("pt-badge pt-badge-x", "X")
    } else {
        ("pt-badge pt-badge-p", "P")
    };
    // X 片段默认展开：它们才是需要关注的分叉点，P 片段保持折叠避免刷屏。
    let open = if is_dynamic { " open" } else { "" };
    write!(
        output,
        "<li class=\"{row_class}\"><details{open}><summary>\
         <span class=\"{badge_class}\">{kind_label}</span>\
         <span class=\"pt-idx\">#{index}</span>\
         <span class=\"pt-excerpt\">{excerpt}</span>\
         <span class=\"pt-size\">{bytes} B</span>\
         <span class=\"pt-sup\"><span class=\"pt-sup-bar\"><i style=\"width:{percent:.0}%\"></i></span>{support}/{members}</span>\
         {warn}</summary><div class=\"pt-body\"><table class=\"pt-fields\">",
        excerpt = encode_text(&excerpt_label(&node.excerpt, 120)),
        bytes = node.utf8_bytes,
        percent = node.support_ratio * 100.0,
        support = node.support_count,
        members = member_count,
        warn = if is_defect {
            "<span class=\"pt-warn\">⚠ 缺陷</span>"
        } else {
            ""
        },
    )?;

    let source = &node.source;
    write!(
        output,
        "<tr><th>来源</th><td><code>{path}</code>{role}</td></tr>",
        path = encode_text(&source.json_path),
        role = match source.role.as_deref() {
            Some(role) => format!("<span class=\"pt-tag\">role={}</span>", encode_text(role)),
            None => String::new(),
        },
    )?;
    let range = match (source.byte_start, source.byte_end) {
        (Some(start), Some(end)) => format!("bytes {start}..{end}"),
        _ => "bytes —".to_string(),
    };
    write!(
        output,
        "<tr><th>位置</th><td><code>{range}</code>{unit}</td></tr>",
        range = encode_text(&range),
        unit = match source.unit_index {
            Some(unit) => format!("<span class=\"pt-tag\">单元 #{unit}</span>"),
            None => String::new(),
        },
    )?;
    write!(
        output,
        "<tr><th>支持度</th><td>{} / {} 个成员（{:.0}%）</td></tr>",
        node.support_count,
        member_count,
        node.support_ratio * 100.0
    )?;
    write!(
        output,
        "<tr><th>摘要</th><td><code>{}</code></td></tr>",
        encode_text(&node.excerpt)
    )?;

    if let Some(variants) = &node.variants {
        if !variants.is_empty() {
            // 按出现次数降序：读者先看到实际发生过的变体，未被该片段采用的排在后面。
            let mut ordered = variants.iter().collect::<Vec<_>>();
            ordered.sort_by(|left, right| right.count.cmp(&left.count));
            output.push_str("<tr><th>变体</th><td><ul class=\"pt-varlist\">");
            for variant in ordered {
                let missing = variant.excerpt.starts_with("（此位置缺失");
                let label = if missing {
                    "此位置缺失".to_string()
                } else {
                    variant.excerpt.clone()
                };
                let (count, zero) = if variant.count_known {
                    (format!("×{}", variant.count), variant.count == 0)
                } else {
                    (format!("×{}（约）", variant.count), false)
                };
                write!(
                    output,
                    "<li class=\"pt-var{}\"><code>{}</code><em>{}{}</em></li>",
                    if missing { " is-missing" } else { "" },
                    encode_text(&label),
                    encode_text(&count),
                    if zero { "（未出现）" } else { "" }
                )?;
            }
            output.push_str("</ul></td></tr>");
        }
    }

    if !node.defect_ids.is_empty() {
        output.push_str("<tr><th>缺陷</th><td>");
        for (position, id) in node.defect_ids.iter().enumerate() {
            if position > 0 {
                output.push('　');
            }
            write!(
                output,
                "<a href=\"#defect-{id}\"><code>{id}</code></a>",
                id = encode_text(id)
            )?;
        }
        output.push_str("</td></tr>");
    }

    if let Some(sample) = &node.content_sample {
        if !sample.is_empty() {
            write!(
                output,
                "<tr><th>内容样本</th><td><pre class=\"pt-sec\">{}</pre></td></tr>",
                encode_text(sample)
            )?;
        }
    }

    let all: std::collections::BTreeSet<&str> = template
        .member_request_ids
        .iter()
        .map(String::as_str)
        .collect();
    let alive: std::collections::BTreeSet<&str> =
        node.member_request_ids.iter().map(String::as_str).collect();
    let lost: Vec<&str> = all.difference(&alive).copied().collect();
    if is_dynamic || !lost.is_empty() {
        output.push_str("<tr><th>成员</th><td class=\"pt-members\">");
        write!(output, "存活 {} / {}：", alive.len(), all.len())?;
        if alive.is_empty() {
            output.push('—');
        } else {
            let alive_list: Vec<&str> = alive.iter().copied().collect();
            render_member_list(output, &alive_list)?;
        }
        if !lost.is_empty() {
            write!(output, "<br>衰减 {}：", lost.len())?;
            render_member_list(output, &lost)?;
        }
        output.push_str("</td></tr>");
    }

    output.push_str("</table></div></details></li>");
    Ok(())
}

fn render_member_list(output: &mut String, ids: &[&str]) -> Result<()> {
    for (index, id) in ids.iter().enumerate() {
        if index == 8 {
            output.push('…');
            break;
        }
        if index > 0 {
            output.push('、');
        }
        write!(output, "<code>{}</code>", encode_text(&short_id(id)))?;
    }
    Ok(())
}

/// 行内摘要：压缩空白并截断，完整文本仍保留在展开体的“摘要”字段。
fn excerpt_label(text: &str, limit: usize) -> String {
    // payload 里的换行多为字面量 `\n`（JSON 转义），一并压平才能让行内摘要保持一行可读。
    let unescaped = text
        .replace("\\r\\n", " ")
        .replace("\\n", " ")
        .replace("\\r", " ")
        .replace("\\t", " ");
    let flat = unescaped.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut label: String = flat.chars().take(limit).collect();
    if flat.chars().count() > limit {
        label.push('…');
    }
    label
}

/// 请求 id 在报告里只需可区分，截断展示避免长串淹没其它字段。
fn short_id(id: &str) -> String {
    let mut short: String = id.chars().take(12).collect();
    if id.chars().count() > 12 {
        short.push('…');
    }
    short
}

#[cfg(test)]
mod tests {
    use actrail_kv_artifacts::{PrefixNode, PrefixNodeKind, TemplatePrefixView};

    use super::*;

    #[test]
    fn prefix_tree_panel_is_rendered_when_view_present() {
        let mut fixture = crate::result_loader::tests::fixture();
        fixture.templates[0].prefix_view = Some(TemplatePrefixView {
            node_count: 2,
            truncated: false,
            nodes: vec![
                PrefixNode {
                    id: "seq-0".into(),
                    kind: PrefixNodeKind::Stable,
                    source: actrail_kv_artifacts::SourceLocation {
                        json_path: "$.messages[0].content".into(),
                        logical_scope: vec![],
                        unit_index: Some(0),
                        byte_start: Some(0),
                        byte_end: Some(8),
                        role: Some("system".into()),
                    },
                    utf8_bytes: 8,
                    support_count: 4,
                    support_ratio: 1.0,
                    excerpt: "PREFIX".into(),
                    member_request_ids: vec!["a".into(), "b".into(), "c".into(), "d".into()],
                    content_sample: Some("PREFIX-sample".into()),
                    sequence_child: Some("seq-1".into()),
                    internal_children: vec![],
                    variants: None,
                    defect_ids: vec![],
                },
                PrefixNode {
                    id: "seq-1".into(),
                    kind: PrefixNodeKind::Dynamic,
                    source: actrail_kv_artifacts::SourceLocation {
                        json_path: "$.messages[0].content".into(),
                        logical_scope: vec![],
                        unit_index: Some(0),
                        byte_start: Some(8),
                        byte_end: Some(9),
                        role: Some("system".into()),
                    },
                    utf8_bytes: 1,
                    support_count: 4,
                    support_ratio: 1.0,
                    excerpt: "1|2|3".into(),
                    member_request_ids: vec!["a".into(), "b".into(), "c".into(), "d".into()],
                    content_sample: None,
                    sequence_child: None,
                    internal_children: vec![],
                    variants: Some(vec![actrail_kv_artifacts::PrefixVariant {
                        excerpt: "1".into(),
                        count: 2,
                        count_known: true,
                    }]),
                    defect_ids: vec!["defect".into()],
                },
            ],
        });
        let html = render_html(&fixture).expect("render html");
        assert!(html.contains("模板前缀树"));
        assert!(html.contains("pt-section"));
        assert!(html.contains("pt-legend"));
        assert!(html.contains("复用分布"));
        assert!(html.contains("成员衰减"));
        assert!(html.contains("结构骨架"));
        // 节点信息拆分为独立字段行，不再堆在一行 meta 里。
        assert!(html.contains("<th>来源</th>"));
        assert!(html.contains("<th>位置</th>"));
        assert!(html.contains("<th>支持度</th>"));
        assert!(html.contains("<th>变体</th>"));
        assert!(html.contains("<th>成员</th>"));
        assert!(html.contains("#defect-defect"));
        assert!(html.contains("PREFIX-sample"));
    }
}
