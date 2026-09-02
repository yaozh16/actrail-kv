//! 本文件将统一 P1/X/P2 缺陷结果渲染为无脚本、全转义的静态 HTML。

use std::{collections::BTreeMap, fmt::Write};

use actrail_kv_artifacts::{AnalysisResult, ContextDefect, DefectFactKind, MismatchPattern};
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
    output.push_str("body{font:15px/1.55 system-ui,sans-serif;margin:2rem;max-width:1100px;color:#17202a}table{border-collapse:collapse;width:100%}th,td{border:1px solid #d5d8dc;padding:.55rem;text-align:left;vertical-align:top}th{background:#f4f6f7}.defect{border:1px solid #ccd1d1;border-radius:8px;padding:1rem;margin:1rem 0}.muted{color:#626567}code{white-space:pre-wrap;overflow-wrap:anywhere}.sequence{font-size:1.05rem}.variant{margin:.5rem 0;padding:.5rem;background:#f8f9f9}</style></head><body>");
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
    output.push_str("</body></html>");
    Ok(output)
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
