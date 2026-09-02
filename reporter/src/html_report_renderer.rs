//! 本文件将结构诊断结果渲染为无脚本、全转义的静态 HTML。

use std::fmt::Write;

use actrail_kv_artifacts::AnalysisResult;
use anyhow::Result;
use html_escape::encode_text;

pub fn render_html(result: &AnalysisResult) -> Result<String> {
    let mut output = String::with_capacity(16_384);
    output.push_str("<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\">");
    output.push_str("<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">");
    output.push_str("<title>Actrail KV 结构诊断</title><style>");
    output.push_str("body{font:15px/1.55 system-ui,sans-serif;margin:2rem;max-width:1100px;color:#17202a}table{border-collapse:collapse;width:100%}th,td{border:1px solid #d5d8dc;padding:.55rem;text-align:left;vertical-align:top}th{background:#f4f6f7}.finding{border:1px solid #ccd1d1;border-radius:8px;padding:1rem;margin:1rem 0}.muted{color:#626567}code{white-space:pre-wrap;overflow-wrap:anywhere}</style></head><body>");
    output.push_str("<h1>LLM KV 缓存结构诊断</h1>");
    write!(
        output,
        "<p>输入 {} 条，分析 {} 条，跳过 {} 条；发现 {} 个聚合问题。</p>",
        result.run.input_records,
        result.run.analyzed_records,
        result.run.skipped_records.len(),
        result.findings.len()
    )?;
    output.push_str("<p class=\"muted\">本报告基于可观察 HTTP payload 的结构代理，不代表真实 Token、KV 命中率或成本收益。</p>");
    output.push_str("<h2>Top K</h2><table><thead><tr><th>排名</th><th>问题</th><th>得分</th><th>受影响请求</th><th>被阻断稳定字节</th></tr></thead><tbody>");
    for entry in &result.top_k {
        write!(
            output,
            "<tr><td>{}</td><td><code>{}</code></td><td>{:.2}</td><td>{}</td><td>{}</td></tr>",
            entry.rank,
            encode_text(&entry.finding_id),
            entry.score.score,
            entry.score.affected_count,
            entry.score.blocked_stable_bytes
        )?;
    }
    output.push_str("</tbody></table><h2>诊断详情</h2>");
    for finding in &result.findings {
        let cause = serde_json::to_string(&finding.cause)?;
        write!(
            output,
            "<section class=\"finding\"><h3>{}</h3><p><strong>位置：</strong><code>{}</code></p><p><strong>原因：</strong>{}</p><p><strong>建议：</strong>{}</p><p><strong>反事实：</strong>{}</p>",
            encode_text(&finding.id),
            encode_text(&finding.source.json_path),
            encode_text(cause.trim_matches('"')),
            encode_text(&finding.recommendation),
            encode_text(&finding.counterfactual)
        )?;
        write!(
            output,
            "<p>实际前缀 {} bytes；潜在前缀 {} bytes；阻断 {} bytes；影响 {} 条；置信度 {:.3}。</p>",
            finding.actual_prefix_bytes,
            finding.potential_prefix_bytes,
            finding.blocked_stable_bytes,
            finding.affected_count,
            finding.confidence
        )?;
        output.push_str("<p><strong>差异证据：</strong></p><ul>");
        for excerpt in &finding.evidence.divergent_excerpts {
            write!(output, "<li><code>{}</code></li>", encode_text(excerpt))?;
        }
        write!(
            output,
            "</ul><p><strong>后续稳定证据：</strong><code>{}</code></p></section>",
            encode_text(&finding.evidence.blocked_stable_excerpt)
        )?;
    }
    output.push_str("</body></html>");
    Ok(output)
}
