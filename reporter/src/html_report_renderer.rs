//! 本文件将统一 P1/X/P2 缺陷结果渲染为无脚本、全转义的静态 HTML。

use std::{collections::BTreeMap, fmt::Write};

use actrail_kv_artifacts::{
    AnalysisResult, CacheMetricBasis, ContextDefect, DefectFactKind, KvCacheMetric,
    MismatchPattern, PrefixNodeKind, SessionEventType,
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
    if let Some(prefix_reuse) = &result.prefix_reuse {
        render_prefix_reuse(&mut output, prefix_reuse, &result.cache_metrics)?;
    }
    render_top_k(&mut output, result, &defects)?;
    output.push_str("<h2>缺陷详情</h2>");
    for defect in &result.defects {
        render_defect(&mut output, defect)?;
    }
    if !result.session_reports.is_empty() {
        render_session_evidence(&mut output, &result.session_reports)?;
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

const SESSION_EVIDENCE_CSS: &str = "\
.sx-tag{display:inline-block;margin-right:.25rem;padding:.05rem .35rem;border-radius:3px;font-size:11px;line-height:1.45;background:#eef2f5;color:#3d566e}\
.sx-append{background:#e8f3fb;color:#1f5c8b}\
.sx-fork{background:#fdf0dd;color:#a35b00}\
.sx-reorder{background:#fdeee0;color:#a04000}\
.sx-reset{background:#fdecea;color:#922b21}\
.sx-bar{display:inline-block;width:52px;height:6px;margin-right:.35rem;border-radius:3px;overflow:hidden;background:#e5e8ea;vertical-align:middle}\
.sx-bar i{display:block;height:100%;background:#2c6fb0}\
.sx-warn .sx-bar i{background:#b9770e}\
.sx-spark{display:inline-flex;align-items:flex-end;gap:1px;height:18px;vertical-align:middle}\
.sx-spark i{display:block;width:5px;min-height:2px;border-radius:1px;background:#2c6fb0}\
.sx-spark i.sx-mid{background:#b9770e}\
.sx-spark i.sx-low{background:#c0392b}\
.sx-range{margin-left:.35rem;color:#626567;font-size:11.5px}\
.sx-detail{margin:1rem 0}\
.sx-detail summary{cursor:pointer;color:#1a5276;font-size:13px}\
";

/// 结构复用率的业务目标（与开发文档的决策一致：写死 90%，不做配置项）。
const PREFIX_REUSE_TARGET: f64 = 0.9;

/// Top K 表格：把已排好序的缺陷翻译成"改哪里、能恢复多少、怎么改"。
///
/// 排序沿用 `top_k[]` 的既有契约（score → affected → blocked → id），不新增排序口径；
/// 建议动作直接复用缺陷自带的 `insights[]`，不在这里新造文案语义。
fn render_top_k(
    output: &mut String,
    result: &AnalysisResult,
    defects: &BTreeMap<&str, &ContextDefect>,
) -> Result<()> {
    output.push_str("<section id=\"top-k\"><h2>Top K</h2>");
    output.push_str(
        "<p class=\"muted\">按 <code>top_k[]</code> 的得分序排列（score → 受影响请求数 → \
         被阻断稳定字节 → 缺陷 ID）；每行给出位置、被阻断稳定字节与建议动作，可跳到下面的证据。\
         被阻断稳定字节是结构代理，不代表真实 Token、KV miss 或金额收益。</p>",
    );
    if result.top_k.is_empty() {
        output.push_str("<p class=\"muted\">本次未发现上下文结构缺陷。</p></section>");
        return Ok(());
    }
    output.push_str(
        "<table class=\"pt-table\"><thead><tr><th>排名</th><th>位置</th><th>被阻断稳定字节</th>\
         <th>受影响请求</th><th>得分</th><th>建议动作</th><th>缺陷</th></tr></thead><tbody>",
    );
    for (index, id) in result.top_k.iter().enumerate() {
        let defect = defects
            .get(id.as_str())
            .ok_or_else(|| anyhow!("top_k references unknown defect {id}"))?;
        let advice = defect
            .insights
            .first()
            .map(|insight| insight.summary.clone())
            .unwrap_or_else(|| "—".to_string());
        write!(
            output,
            "<tr><td>{rank}</td><td><code>{location}</code></td><td>{blocked}</td>\
             <td>{affected}</td><td>{score:.2}</td><td>{advice}</td>\
             <td><a href=\"#defect-{anchor}\"><code>{id}</code></a></td></tr>",
            rank = index + 1,
            location = encode_text(&defect_location(defect)),
            blocked = defect.blocked_stable_bytes,
            affected = defect.affected_count,
            score = defect.score.score,
            advice = encode_text(&advice),
            anchor = encode_text(id),
            id = encode_text(id),
        )?;
    }
    output.push_str("</tbody></table></section>");
    Ok(())
}

/// 机会清单里的位置：取第一个变体代表证据的来源，附角色与投影单元序号。
fn defect_location(defect: &ContextDefect) -> String {
    let source = defect
        .mismatch
        .variants
        .iter()
        .find_map(|variant| variant.representative.sources.first());
    let Some(source) = source else {
        return "—".to_string();
    };
    let mut location = source.json_path.clone();
    if let Some(role) = source.role.as_deref() {
        write!(location, " ({role})").expect("writing into String cannot fail");
    }
    if let Some(unit) = source.unit_index {
        write!(location, " · 单元 #{unit}").expect("writing into String cannot fail");
    }
    location
}

const PREFIX_REUSE_CSS: &str = "\
.pr-scale{position:relative;height:24px;margin:.5rem 0 .6rem;border:1px solid #d5d8dc;border-radius:4px;background:#f4f6f7;overflow:hidden}\
.pr-scale i{position:absolute;top:0;bottom:0;display:block}\
.pr-now{left:0;background:#2c6fb0}\
.pr-gain{background:#e6a23c}\
.pr-new{background:#dfe4e8}\
.pr-target{position:absolute;top:0;bottom:0;width:2px;background:#c0392b}\
.pr-legend{display:flex;flex-wrap:wrap;gap:1.2rem;font-size:13px}\
.pr-note{margin:.35rem 0 .8rem;color:#626567;font-size:12.5px}\
";

/// 首屏结论：当前结构复用率、补完已知结构问题后的上界、以及 90% 目标。
fn render_prefix_reuse(
    output: &mut String,
    summary: &actrail_kv_artifacts::PrefixReuseSummary,
    cache_metrics: &[KvCacheMetric],
) -> Result<()> {
    let current = summary.ratio.clamp(0.0, 1.0);
    let potential = summary.potential_ratio.clamp(current, 1.0);
    let target = PREFIX_REUSE_TARGET.clamp(0.0, 1.0);
    output.push_str("<section id=\"prefix-reuse\"><h2>结构复用率</h2><style>");
    output.push_str(PREFIX_REUSE_CSS);
    output.push_str("</style>");
    output.push_str(
        "<p class=\"muted\">口径：会话内相邻请求对的公共前缀字节 / 上下文总字节。\
         这是结构代理，不是真实 KV 命中率，也不代表 Token 或金额收益。</p>",
    );
    write!(
        output,
        "<div class=\"pr-scale\"><i class=\"pr-now\" style=\"width:{now:.2}%\"></i>\
         <i class=\"pr-gain\" style=\"left:{now:.2}%;width:{gain:.2}%\"></i>\
         <i class=\"pr-new\" style=\"left:{potential:.2}%;width:{new_content:.2}%\"></i>\
         <span class=\"pr-target\" style=\"left:{target:.2}%\"></span></div>",
        now = current * 100.0,
        gain = (potential - current) * 100.0,
        potential = potential * 100.0,
        new_content = (1.0 - potential) * 100.0,
        target = target * 100.0,
    )?;
    write!(
        output,
        "<div class=\"pr-legend\"><span><b>当前</b> {:.1}%</span>\
         <span><b>补完已知结构问题</b> {:.1}%（可恢复 {:.1} KB）</span>\
         <span><b>目标</b> {:.0}%</span></div>",
        current * 100.0,
        potential * 100.0,
        summary.gain_bytes as f64 / 1024.0,
        target * 100.0,
    )?;
    let composition = &summary.composition;
    write!(
        output,
        "<p class=\"pr-note\">字节构成：已复用 {} · 已知可恢复 {} · 每轮新增 {}（会话首个请求没有前序，不进入统计）。</p>",
        human_bytes(composition.reusable_bytes),
        human_bytes(composition.recoverable_bytes),
        human_bytes(composition.new_content_bytes),
    )?;
    if current >= target {
        write!(
            output,
            "<p class=\"pr-note\">已高于目标 {:.1} 个百分点；覆盖 {} 个会话的 {} 个相邻请求对。</p>",
            (current - target) * 100.0,
            summary.sessions,
            summary.request_pairs
        )?;
    } else if potential >= target {
        write!(
            output,
            "<p class=\"pr-note\">距目标 {:.1} 个百分点；补完已知结构问题后可越过目标。\
             覆盖 {} 个会话的 {} 个相邻请求对。</p>",
            (target - current) * 100.0,
            summary.sessions,
            summary.request_pairs
        )?;
    } else {
        write!(
            output,
            "<p class=\"pr-note\">距目标 {:.1} 个百分点；补完已知结构问题后到 {:.1}%，仍差 {:.1} 个百分点。\
             剩余 {:.1}% 的上下文是每轮新增内容，按前缀缓存口径不可复用——\
             要再往上走需要减少每轮新增量，而不是继续调整内容顺序。\
             覆盖 {} 个会话的 {} 个相邻请求对。</p>",
            (target - current) * 100.0,
            potential * 100.0,
            (target - potential) * 100.0,
            (1.0 - potential) * 100.0,
            summary.sessions,
            summary.request_pairs
        )?;
    }
    render_proxy_cross_check(output, summary, cache_metrics)?;
    output.push_str("</section>");
    Ok(())
}

/// 逐会话把结构复用率与真实命中率放在一起，用来交叉验证结构代理的方向是否可信。
#[derive(Default)]
struct SessionHitAggregate {
    prompt_tokens: u64,
    cached_tokens: u64,
    reported: usize,
    estimated: usize,
}

/// 交叉验证表最多展示的会话数：超出时只列差异最大的若干个。
const MAX_CROSS_CHECK_ROWS: usize = 8;

fn render_proxy_cross_check(
    output: &mut String,
    summary: &actrail_kv_artifacts::PrefixReuseSummary,
    cache_metrics: &[KvCacheMetric],
) -> Result<()> {
    if cache_metrics.is_empty() || summary.by_session.is_empty() {
        return Ok(());
    }
    let mut aggregates: BTreeMap<&str, SessionHitAggregate> = BTreeMap::new();
    for metric in cache_metrics {
        let Some(session_key) = metric.session_key.as_deref() else {
            continue;
        };
        let aggregate = aggregates.entry(session_key).or_default();
        match metric.basis {
            CacheMetricBasis::Reported => {
                aggregate.reported += 1;
                aggregate.prompt_tokens += u64::from(metric.prompt_tokens.unwrap_or(0));
                aggregate.cached_tokens += u64::from(metric.cached_tokens.unwrap_or(0));
            }
            CacheMetricBasis::Estimated => aggregate.estimated += 1,
        }
    }
    // (会话, 结构复用率, 真实命中率, 真实样本数, 估算样本数)
    let mut rows: Vec<(&str, f64, Option<f64>, usize, usize)> = Vec::new();
    for session in &summary.by_session {
        let Some(aggregate) = aggregates.get(session.session_key.as_str()) else {
            continue;
        };
        let real_hit_rate = (aggregate.reported > 0 && aggregate.prompt_tokens > 0)
            .then(|| aggregate.cached_tokens as f64 / aggregate.prompt_tokens as f64);
        rows.push((
            session.session_key.as_str(),
            session.ratio,
            real_hit_rate,
            aggregate.reported,
            aggregate.estimated,
        ));
    }
    if rows.is_empty() {
        return Ok(());
    }
    output.push_str("<h3>结构代理 vs 真实命中</h3>");
    output.push_str(
        "<p class=\"muted\">结构复用率按字节口径（公共前缀 / 上下文），真实命中率按 token 口径\
         （cached / prompt）；两者口径不同，只用来判断方向是否一致，不表示两者应当相等。\
         估算口径的请求不参与对比。</p>",
    );
    let estimated_only = rows.len() - rows.iter().filter(|row| row.2.is_some()).count();
    if estimated_only == rows.len() {
        write!(
            output,
            "<p class=\"muted\">本次没有采集到上游 usage（{} 个会话全部为估算口径），\
             无法与真实命中率交叉验证。</p>",
            rows.len()
        )?;
        return Ok(());
    }
    // 只有真实命中率可以参与对比；其余会话数量单列提示。
    rows.retain(|row| row.2.is_some());
    // 差异越大越值得先看。
    rows.sort_by(|left, right| {
        let left_delta = (left.1 - left.2.unwrap_or(0.0)).abs();
        let right_delta = (right.1 - right.2.unwrap_or(0.0)).abs();
        right_delta
            .partial_cmp(&left_delta)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.0.cmp(right.0))
    });
    let total = rows.len();
    let truncated = total > MAX_CROSS_CHECK_ROWS;
    rows.truncate(MAX_CROSS_CHECK_ROWS);
    if estimated_only > 0 {
        write!(
            output,
            "<p class=\"muted\">另有 {} 个会话没有真实 usage，未参与对比。</p>",
            estimated_only
        )?;
    }
    if truncated {
        write!(
            output,
            "<p class=\"muted\">仅列出差异最大的 {} 个会话（共 {} 个可比会话）。</p>",
            MAX_CROSS_CHECK_ROWS, total
        )?;
    }
    output.push_str(
        "<table class=\"pt-table\"><thead><tr><th>会话</th><th>结构复用率（字节）</th>\
         <th>真实命中率（token）</th><th>差值</th><th>口径覆盖</th></tr></thead><tbody>",
    );
    for (session_key, structural, real_hit_rate, reported, estimated) in &rows {
        let real_cell = match real_hit_rate {
            Some(rate) => format!("{:.1}%", rate * 100.0),
            None => "—（全部为估算口径）".to_string(),
        };
        let delta_cell = match real_hit_rate {
            Some(rate) => format!("{:+.1}pp", (structural - rate) * 100.0),
            None => "—".to_string(),
        };
        write!(
            output,
            "<tr><td><code>{session}</code></td><td>{structural:.1}%</td><td>{real_cell}</td>\
             <td>{delta_cell}</td><td>真实 {reported} / 估算 {estimated}</td></tr>",
            session = encode_text(session_key),
            structural = structural * 100.0,
        )?;
    }
    output.push_str("</tbody></table>");
    Ok(())
}

/// 报告里的字节量按 KB / MB 展示，避免长数字串。
fn human_bytes(bytes: usize) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    let value = bytes as f64;
    if value >= MB {
        format!("{:.1} MB", value / MB)
    } else if value >= KB {
        format!("{:.0} KB", value / KB)
    } else {
        format!("{bytes} B")
    }
}

/// 会话前缀切换证据：先给整体结论，再只列需要关注的会话，其余折叠。
fn render_session_evidence(
    output: &mut String,
    reports: &[actrail_kv_artifacts::SessionReport],
) -> Result<()> {
    output.push_str("<h2>会话前缀切换证据</h2><style>");
    output.push_str(SESSION_EVIDENCE_CSS);
    output.push_str("</style>");
    output.push_str(
        "<p class=\"muted\">同一会话里相邻两个请求的公共前缀，就是上一轮上下文里能被直接复用的部分；\
         前缀收缩、重排或重置会让其后的内容整体重算。复用率 = 公共前缀字节 / 该请求上下文总字节。</p>",
    );

    let mut requests = 0usize;
    let mut lcp_bytes = 0usize;
    let mut next_bytes = 0usize;
    let mut recoverable = 0usize;
    let mut cut_sessions = 0usize;
    for report in reports {
        requests += report.request_count;
        recoverable += report.total_stable_after_switch_bytes;
        if report.total_prefix_cut_bytes > 0 {
            cut_sessions += 1;
        }
        for event in &report.events {
            lcp_bytes += event.lcp_bytes;
            next_bytes += event.next_total_bytes;
        }
    }
    let reuse_ratio = if next_bytes == 0 {
        0.0
    } else {
        lcp_bytes as f64 / next_bytes as f64
    };
    let common_prefix = common_key_prefix(reports.iter().map(|report| report.session_key.as_str()));
    let endpoints: std::collections::BTreeSet<&str> = reports
        .iter()
        .map(|report| report.endpoint_key.as_str())
        .collect();
    let models: std::collections::BTreeSet<&str> =
        reports.iter().map(|report| report.model.as_str()).collect();
    let mut meta = format!(
        "端点 <code>{}</code> · 模型 <code>{}</code>",
        encode_text(&endpoints.into_iter().collect::<Vec<_>>().join(" / ")),
        encode_text(&models.into_iter().collect::<Vec<_>>().join(" / "))
    );
    if !common_prefix.is_empty() {
        write!(
            meta,
            " · 会话列省略共有前缀 <code>{}</code>（完整会话名见 analysis.json）",
            encode_text(&common_prefix)
        )?;
    }
    write!(output, "<p class=\"muted\">{meta}</p>")?;
    write!(
        output,
        "<div class=\"pt-summary\"><div class=\"pt-metric\"><b>{}</b>会话</div>\
         <div class=\"pt-metric\"><b>{}</b>请求</div>\
         <div class=\"pt-metric\"><b>{:.1}%</b>复用前缀占比</div>\
         <div class=\"pt-metric\"><b>{}</b>存在前缀收缩</div>\
         <div class=\"pt-metric\"><b>{}</b> B</div></div>",
        reports.len(),
        requests,
        reuse_ratio * 100.0,
        cut_sessions,
        recoverable
    )?;
    output.push_str(
        "<p class=\"muted\">末项为可恢复稳定字节：前缀收缩后仍然稳定、本可复用却被丢弃的内容。</p>",
    );

    let mut attention: Vec<&actrail_kv_artifacts::SessionReport> = reports
        .iter()
        .filter(|report| session_needs_attention(report))
        .collect();
    let mut normal: Vec<&actrail_kv_artifacts::SessionReport> = reports
        .iter()
        .filter(|report| !session_needs_attention(report))
        .collect();
    attention.sort_by(|left, right| {
        session_reuse_ratio(left)
            .partial_cmp(&session_reuse_ratio(right))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    normal.sort_by(|left, right| left.session_key.cmp(&right.session_key));

    if attention.is_empty() {
        output
            .push_str("<p class=\"muted\">所有会话都是追加式增长，没有前缀收缩、重排或重置。</p>");
    } else {
        write!(output, "<h3>需要关注（{}）</h3>", attention.len())?;
        render_session_table(output, &attention, true, &common_prefix)?;
    }
    if !normal.is_empty() {
        // 多轮会话的逐轮明细值得摊开；两轮会话只有一个事件，行内数字已经够看。
        let include_detail = normal.iter().any(|report| report.events.len() > 1);
        // 会话少时直接列出，避免"全是正常"时整段只剩一句话；会话多时才折叠。
        const OPEN_LIMIT: usize = 8;
        if normal.len() <= OPEN_LIMIT {
            write!(output, "<h3>会话明细（{}）</h3>", normal.len())?;
            render_session_table(output, &normal, include_detail, &common_prefix)?;
        } else {
            write!(
                output,
                "<details class=\"sx-detail\"><summary>查看其余 {} 个会话（均为追加式增长）</summary>",
                normal.len()
            )?;
            render_session_table(output, &normal, include_detail, &common_prefix)?;
            output.push_str("</details>");
        }
    }
    Ok(())
}

/// 所有会话名共有的前缀（通常是批次/运行名），展示时省略。
fn common_key_prefix<'a>(keys: impl Iterator<Item = &'a str>) -> String {
    let mut common: Option<Vec<char>> = None;
    for key in keys {
        let chars: Vec<char> = key.chars().collect();
        common = Some(match common {
            None => chars,
            Some(current) => {
                let mut index = 0;
                while index < current.len() && index < chars.len() && current[index] == chars[index]
                {
                    index += 1;
                }
                current[..index].to_vec()
            }
        });
    }
    common.unwrap_or_default().into_iter().collect()
}

/// 会话标签：去掉公共前缀后只留可区分的尾部；全部被剥掉时退回截断的原名。
fn session_label(key: &str, common_prefix: &str) -> String {
    let stripped = key
        .strip_prefix(common_prefix)
        .unwrap_or(key)
        .trim_start_matches(['-', '_', '/']);
    // 尾部太短（例如只剩 "-s1" 里的 "1"）不足以单独辨识，改回展示原名尾部。
    let label = if stripped.chars().count() >= 8 {
        stripped.to_string()
    } else {
        let tail: Vec<char> = key.chars().collect();
        let start = tail.len().saturating_sub(20);
        format!("…{}", tail[start..].iter().collect::<String>())
    };
    let mut shown: String = label.chars().take(32).collect();
    if label.chars().count() > 32 {
        shown.push('…');
    }
    shown
}

/// 会话是否值得单独提示：出现非追加事件，或存在前缀收缩。
fn session_needs_attention(report: &actrail_kv_artifacts::SessionReport) -> bool {
    report.fork_count > 0
        || report.reorder_count > 0
        || report.reset_count > 0
        || report.total_prefix_cut_bytes > 0
}

fn session_reuse_ratio(report: &actrail_kv_artifacts::SessionReport) -> f64 {
    let lcp: usize = report.events.iter().map(|event| event.lcp_bytes).sum();
    let total: usize = report
        .events
        .iter()
        .map(|event| event.next_total_bytes)
        .sum();
    if total == 0 {
        0.0
    } else {
        lcp as f64 / total as f64
    }
}

fn render_session_table(
    output: &mut String,
    reports: &[&actrail_kv_artifacts::SessionReport],
    include_detail: bool,
    common_prefix: &str,
) -> Result<()> {
    output.push_str(
        "<table class=\"pt-table\"><thead><tr><th>会话</th><th>请求</th><th>事件</th>\
         <th>复用前缀</th><th>复用率</th><th>逐轮</th><th>前缀收缩</th><th>可恢复稳定</th>",
    );
    if include_detail {
        output.push_str("<th>明细</th>");
    }
    output.push_str("</tr></thead><tbody>");
    for report in reports {
        let lcp: usize = report.events.iter().map(|event| event.lcp_bytes).sum();
        let total: usize = report
            .events
            .iter()
            .map(|event| event.next_total_bytes)
            .sum();
        let ratio = session_reuse_ratio(report);
        let warn = report.total_prefix_cut_bytes > 0
            || report.fork_count > 0
            || report.reorder_count > 0
            || report.reset_count > 0;
        write!(
            output,
            "<tr class=\"{row_class}\"><td><code>{session}</code></td><td>{requests}</td><td>{tags}</td>\
             <td>{lcp} B / {total} B</td>\
             <td><span class=\"sx-bar\"><i style=\"width:{percent:.0}%\"></i></span>{percent:.0}%</td>\
             <td>{spark}</td>\
             <td>{cut} B</td><td>{recoverable} B</td>{detail}</tr>",
            row_class = if warn { "sx-warn" } else { "" },
            session = encode_text(&session_label(&report.session_key, common_prefix)),
            requests = report.request_count,
            tags = session_event_tags(report),
            percent = ratio * 100.0,
            spark = session_reuse_spark(report),
            cut = report.total_prefix_cut_bytes,
            recoverable = report.total_stable_after_switch_bytes,
            detail = if include_detail {
                format!("<td>{}</td>", session_event_detail(report))
            } else {
                String::new()
            },
        )?;
    }
    output.push_str("</tbody></table>");
    Ok(())
}

/// 逐轮复用率迷你柱：多轮会话的走势（每注入一段新内容就掉一次、随后回升）只有摊开才看得出。
fn session_reuse_spark(report: &actrail_kv_artifacts::SessionReport) -> String {
    if report.events.is_empty() {
        return "—".to_string();
    }
    let mut bars = String::from("<span class=\"sx-spark\">");
    let mut lowest = f64::MAX;
    let mut highest = f64::MIN;
    for event in &report.events {
        let ratio = if event.next_total_bytes == 0 {
            0.0
        } else {
            event.lcp_bytes as f64 / event.next_total_bytes as f64
        };
        lowest = lowest.min(ratio);
        highest = highest.max(ratio);
        let class = if ratio < 0.5 {
            "sx-low"
        } else if ratio < 0.75 {
            "sx-mid"
        } else {
            "sx-high"
        };
        write!(
            bars,
            "<i class=\"{class}\" style=\"height:{height:.0}%\"></i>",
            height = (ratio * 100.0).clamp(8.0, 100.0)
        )
        .expect("writing into String cannot fail");
    }
    bars.push_str("</span>");
    write!(
        bars,
        "<span class=\"sx-range\">{:.0}%~{:.0}%</span>",
        lowest * 100.0,
        highest * 100.0
    )
    .expect("writing into String cannot fail");
    bars
}

fn session_event_tags(report: &actrail_kv_artifacts::SessionReport) -> String {
    let mut tags = String::new();
    for (count, class, label) in [
        (report.append_count, "sx-append", "追加"),
        (report.fork_count, "sx-fork", "分叉"),
        (report.reorder_count, "sx-reorder", "重排"),
        (report.reset_count, "sx-reset", "重置"),
    ] {
        if count > 0 {
            write!(
                tags,
                "<span class=\"sx-tag {class}\">{label}×{count}</span>"
            )
            .expect("writing into String cannot fail");
        }
    }
    if tags.is_empty() {
        "—".to_string()
    } else {
        tags
    }
}

/// 逐事件明细默认折叠：只在这里展示请求 id、重算量与块结构。
fn session_event_detail(report: &actrail_kv_artifacts::SessionReport) -> String {
    let mut detail = String::from(
        "<details><summary>事件</summary><table class=\"pt-table\"><thead><tr>\
         <th>#</th><th>类型</th><th>前一请求</th><th>后一请求</th><th>公共前缀</th>\
         <th>收缩</th><th>重算</th><th>可恢复稳定</th><th>块结构</th></tr></thead><tbody>",
    );
    for (index, event) in report.events.iter().enumerate() {
        write!(
            detail,
            "<tr><td>{}</td><td>{}</td><td><code>{}</code></td><td><code>{}</code></td>\
             <td>{} units / {} B</td><td>{} B</td><td>{} B</td><td>{} B</td><td><code>{}</code></td></tr>",
            index + 1,
            event_label(&event.event_type),
            encode_text(&short_id(&event.prev_request_id)),
            encode_text(&short_id(&event.next_request_id)),
            event.lcp_units,
            event.lcp_bytes,
            event.prefix_cut_bytes,
            event.recomputed_bytes,
            event.stable_after_switch_bytes,
            encode_text(&event.next_block_runs)
        )
        .expect("writing into String cannot fail");
    }
    detail.push_str("</tbody></table></details>");
    detail
}

fn event_label(event: &SessionEventType) -> &'static str {
    match event {
        SessionEventType::Append => "追加",
        SessionEventType::Fork => "分叉",
        SessionEventType::Reorder => "重排",
        SessionEventType::Reset => "重置",
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
        "<section class=\"defect\" id=\"defect-{}\"><h3>{}</h3>\
         <p class=\"muted\"><a href=\"#top-k\">↑ 返回 Top K</a></p>\
         <p><strong>比较组：</strong><code>window={} endpoint={} model={} schema={}</code></p>",
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
    use actrail_kv_artifacts::{
        PrefixNode, PrefixNodeKind, PrefixReuseSummary, SessionEventType, SessionReport,
        SessionSwitchEvent, TemplatePrefixView,
    };

    use super::*;

    fn prefix_reuse(ratio: f64, potential_ratio: f64) -> PrefixReuseSummary {
        PrefixReuseSummary {
            sessions: 2,
            request_pairs: 8,
            lcp_bytes: 840,
            context_bytes: 1000,
            ratio,
            potential_lcp_bytes: (potential_ratio * 1000.0) as usize,
            potential_ratio,
            gain_bytes: 90,
            recoverable_stable_bytes: 0,
            sessions_with_prefix_cut: 0,
            by_session: vec![actrail_kv_artifacts::PrefixReuseSession {
                session_key: "session-a".to_string(),
                request_pairs: 4,
                lcp_bytes: (ratio * 500.0) as usize,
                context_bytes: 500,
                ratio,
                potential_lcp_bytes: (potential_ratio * 500.0) as usize,
                gain_bytes: ((potential_ratio - ratio) * 500.0) as usize,
            }],
            requests: vec![],
            composition: actrail_kv_artifacts::PrefixReuseComposition {
                context_bytes: 1000,
                reusable_bytes: (ratio * 1000.0) as usize,
                recoverable_bytes: ((potential_ratio - ratio) * 1000.0) as usize,
                new_content_bytes: ((1.0 - potential_ratio) * 1000.0) as usize,
            },
        }
    }

    fn cache_metric(session: &str, basis: CacheMetricBasis) -> KvCacheMetric {
        KvCacheMetric {
            request_id: format!("{session}:0"),
            session_key: Some(session.to_string()),
            sequence: 1,
            captured_at: None,
            source: None,
            prompt_tokens: Some(1000),
            cached_tokens: Some(700),
            miss_tokens: Some(300),
            output_tokens: Some(10),
            hit_rate: 0.7,
            hit_rate_delta: None,
            basis,
            estimated_lcp_bytes: None,
            payload_bytes: 1200,
            ttft_ms: None,
            total_ms: None,
        }
    }

    #[test]
    fn cross_check_pairs_structural_reuse_with_reported_hit_rate() {
        let mut fixture = crate::result_loader::tests::fixture();
        fixture.prefix_reuse = Some(prefix_reuse(0.84, 0.93));
        fixture.cache_metrics = vec![cache_metric("session-a", CacheMetricBasis::Reported)];
        let html = render_html(&fixture).expect("render html");
        assert!(html.contains("结构代理 vs 真实命中"));
        assert!(html.contains("70.0%"), "真实命中率应来自 token 口径");
        assert!(html.contains("+14.0pp"), "差值应为结构与真实之差");

        // 只有估算口径时不能伪装成真实命中率。
        let mut estimated = crate::result_loader::tests::fixture();
        estimated.prefix_reuse = Some(prefix_reuse(0.84, 0.93));
        estimated.cache_metrics = vec![cache_metric("session-a", CacheMetricBasis::Estimated)];
        let html = render_html(&estimated).expect("render html");
        let block = &html[html
            .find("结构代理 vs 真实命中")
            .expect("cross check section")..];
        assert!(block.contains("无法与真实命中率交叉验证"));
        // 交叉验证区块本身不能出现"真实命中率"列，估算值只能留在 KV 命中率区块里。
        assert!(!block.contains("<th>真实命中率（token）</th>"));
    }

    #[test]
    fn top_k_table_links_each_defect_to_its_evidence() {
        let fixture = crate::result_loader::tests::fixture();
        let html = render_html(&fixture).expect("render html");
        assert!(html.contains("<h2>Top K</h2>"));
        // 位置、阻断量、受影响请求数、建议动作与证据锚点都要出现在清单里。
        assert!(html.contains("$.messages[0].content"));
        assert!(html.contains("被阻断稳定字节"));
        assert!(html.contains("&lt;b&gt;统一生成方式&lt;/b&gt;"));
        assert!(html.contains("href=\"#defect-defect\""));
        assert!(html.contains("id=\"defect-defect\""));
        // 排序沿用 top_k[]，不是重新排序。
        assert!(html.contains("按 <code>top_k[]</code> 的得分序排列"));

        // 没有缺陷时给出空态，而不是渲染空表。
        let mut empty = crate::result_loader::tests::fixture();
        empty.defects.clear();
        empty.top_k.clear();
        let html = render_html(&empty).expect("render html");
        assert!(html.contains("本次未发现上下文结构缺陷"));
    }

    #[test]
    fn prefix_reuse_scale_reports_current_upper_bound_and_target() {
        let mut fixture = crate::result_loader::tests::fixture();
        fixture.prefix_reuse = Some(prefix_reuse(0.84, 0.93));
        let html = render_html(&fixture).expect("render html");
        assert!(html.contains("结构复用率"));
        assert!(html.contains("84.0%"));
        assert!(html.contains("93.0%"));
        assert!(html.contains("<b>目标</b> 90%"));
        assert!(html.contains("补完已知结构问题后可越过目标"));
        assert!(html.contains("pr-scale"));
        // 三段构成：已复用 / 已知可恢复 / 每轮新增
        assert!(html.contains("字节构成"));
        assert!(html.contains("pr-new"));

        // 补完已知结构问题仍不达标时，要给出剩余差距。
        let mut short = crate::result_loader::tests::fixture();
        short.prefix_reuse = Some(prefix_reuse(0.50, 0.70));
        let html = render_html(&short).expect("render html");
        assert!(html.contains("仍差 20.0 个百分点"));

        // 已经达标时不再提示补完。
        let mut done = crate::result_loader::tests::fixture();
        done.prefix_reuse = Some(prefix_reuse(0.95, 0.97));
        let html = render_html(&done).expect("render html");
        assert!(html.contains("已高于目标 5.0 个百分点"));

        // 没有汇总时整块不渲染。
        let mut absent = crate::result_loader::tests::fixture();
        absent.prefix_reuse = None;
        let html = render_html(&absent).expect("render html");
        assert!(!html.contains("结构复用率"));
    }

    fn session_report(
        key: &str,
        event_type: SessionEventType,
        prefix_cut_bytes: usize,
    ) -> SessionReport {
        let append = matches!(event_type, SessionEventType::Append);
        let reset = matches!(event_type, SessionEventType::Reset);
        SessionReport {
            session_key: key.to_string(),
            endpoint_key: "endpoint-a".to_string(),
            model: "model-a".to_string(),
            request_count: 2,
            append_count: usize::from(append),
            fork_count: 0,
            reorder_count: 0,
            reset_count: usize::from(reset),
            total_recomputed_bytes: 100,
            total_stable_after_switch_bytes: 4,
            total_prefix_cut_bytes: prefix_cut_bytes,
            avg_prefix_cut_bytes: prefix_cut_bytes,
            events: vec![SessionSwitchEvent {
                event_type,
                prev_request_id: "prev-request-id".to_string(),
                next_request_id: "next-request-id".to_string(),
                lcp_units: 3,
                lcp_bytes: 900,
                previous_pair_lcp_bytes: 0,
                prefix_cut_bytes,
                next_total_bytes: 1000,
                recomputed_bytes: 100,
                stable_after_switch_bytes: 4,
                next_block_runs: "abcdx1".to_string(),
            }],
        }
    }

    #[test]
    fn session_evidence_groups_attention_sessions_and_strips_common_prefix() {
        let mut fixture = crate::result_loader::tests::fixture();
        fixture.session_reports = vec![
            session_report("batch-1-normal-01", SessionEventType::Append, 0),
            session_report("batch-1-reset-01", SessionEventType::Reset, 0),
        ];
        let html = render_html(&fixture).expect("render html");
        assert!(html.contains("会话前缀切换证据"));
        assert!(html.contains("复用前缀占比"));
        assert!(html.contains("90.0%"));
        // 只把非追加事件列进关注区；会话少时其余会话直接铺开
        assert!(html.contains("需要关注（1）"));
        assert!(html.contains("会话明细（1）"));
        assert!(!html.contains("查看其余"));
        // 会话列去掉公共前缀，只留可区分部分
        assert!(html.contains(">reset-01<"));
        assert!(!html.contains("batch-1-reset-01"));
        assert!(html.contains("batch-1-"));
        // 事件类型用中文标签
        assert!(html.contains("追加×1"));
        assert!(html.contains("重置×1"));
        // 明细折叠在行内，不再一个会话一个区块
        assert!(html.contains("<summary>事件</summary>"));
        assert_eq!(html.matches("会话前缀切换证据").count(), 1);
        // 逐轮迷你柱与区间
        assert!(html.contains("sx-spark"));
        assert!(html.contains("sx-range"));
    }

    #[test]
    fn session_evidence_collapses_many_healthy_sessions() {
        let mut fixture = crate::result_loader::tests::fixture();
        fixture.session_reports = (1..=9)
            .map(|index| {
                session_report(
                    &format!("batch-1-normal-{index:02}"),
                    SessionEventType::Append,
                    0,
                )
            })
            .collect();
        let html = render_html(&fixture).expect("render html");
        assert!(!html.contains("需要关注"));
        assert!(html.contains("所有会话都是追加式增长"));
        assert!(html.contains("查看其余 9 个会话"));
    }

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
