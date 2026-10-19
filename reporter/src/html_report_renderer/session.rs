//! 本文件渲染会话上下文刷新、逐轮前缀复用度与刷新事件证据。

use std::fmt::Write;

use actrail_kv_artifacts::{SessionEventType, SessionReport, SessionSwitchEvent};
use anyhow::Result;
use html_escape::encode_text;

use super::short_id;

const SESSION_EVIDENCE_CSS: &str = "\
.sx-tag{display:inline-block;margin-right:.25rem;padding:.05rem .35rem;border-radius:3px;font-size:11px;line-height:1.45;background:#eef2f5;color:#3d566e}\
.sx-append{background:#e8f3fb;color:#1f5c8b}\
.sx-fork{background:#fdf0dd;color:#a35b00}\
.sx-reorder{background:#fdeee0;color:#a04000}\
.sx-reset{background:#fdecea;color:#922b21}\
.sx-bar{display:inline-block;width:52px;height:6px;margin-right:.35rem;border-radius:3px;overflow:hidden;background:#e5e8ea;vertical-align:middle}\
.sx-bar i{display:block;height:100%;background:#2c6fb0}\
.sx-warn .sx-bar i{background:#b9770e}\
.sx-warn{background:#fff8e8}\
.sx-spark{display:inline-flex;align-items:flex-end;gap:1px;height:18px;vertical-align:middle}\
.sx-spark i{display:block;width:5px;min-height:2px;border-radius:1px;background:#2c6fb0}\
.sx-spark i.sx-mid{background:#b9770e}\
.sx-spark i.sx-low{background:#c0392b}\
.sx-range{margin-left:.35rem;color:#626567;font-size:11.5px}\
.sx-detail{margin:1rem 0}\
.sx-detail summary{cursor:pointer;color:#1a5276;font-size:13px}\
";

pub(super) fn render_session_evidence(
    output: &mut String,
    reports: &[SessionReport],
) -> Result<()> {
    output.push_str("<h2>会话上下文刷新与前缀复用</h2><style>");
    output.push_str(SESSION_EVIDENCE_CSS);
    output.push_str("</style>");
    output.push_str(
        "<p class=\"muted\">同一 session 内的相邻请求本应保持 append-only。\
         每轮前缀复用度 = 与上一条请求的公共前缀字节 / 上一条内容性上下文总字节；\
         低于 100% 表示上一条上下文未被完整保留，会废弃原有 KVC 前缀。</p>",
    );

    let mut requests = 0usize;
    let mut lcp_bytes = 0usize;
    let mut next_bytes = 0usize;
    let mut refresh_count = 0usize;
    let mut refresh_sessions = 0usize;
    let mut abandoned_bytes = 0usize;
    for report in reports {
        requests += report.request_count;
        let report_refreshes = effective_refresh_count(report);
        refresh_count += report_refreshes;
        refresh_sessions += usize::from(report_refreshes > 0);
        abandoned_bytes += effective_abandoned_prefix_bytes(report);
        for event in &report.events {
            lcp_bytes += event.lcp_bytes;
            next_bytes += event.next_total_bytes;
        }
    }
    let reuse_ratio = ratio(lcp_bytes, next_bytes);
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
         <div class=\"pt-metric\"><b>{}</b>上下文刷新警告</div>\
         <div class=\"pt-metric\"><b>{}</b>发生刷新的会话</div>\
         <div class=\"pt-metric\"><b>{:.1}%</b>当前前缀复用占比</div>\
         <div class=\"pt-metric\"><b>{}</b> B</div></div>",
        reports.len(),
        requests,
        refresh_count,
        refresh_sessions,
        reuse_ratio * 100.0,
        abandoned_bytes
    )?;
    output.push_str(
        "<p class=\"muted\">末项为废弃前缀字节。上下文压缩、subagent 创建或任务切换都可能触发刷新；\
         这些是 KVC 前缀废弃风险，不自动判定为错误，由用户确认是否接受。</p>",
    );

    let mut attention: Vec<&SessionReport> = reports
        .iter()
        .filter(|report| effective_refresh_count(report) > 0)
        .collect();
    let mut normal: Vec<&SessionReport> = reports
        .iter()
        .filter(|report| effective_refresh_count(report) == 0)
        .collect();
    attention.sort_by(|left, right| {
        effective_refresh_count(right)
            .cmp(&effective_refresh_count(left))
            .then_with(|| {
                effective_abandoned_prefix_bytes(right).cmp(&effective_abandoned_prefix_bytes(left))
            })
            .then_with(|| left.session_key.cmp(&right.session_key))
    });
    normal.sort_by(|left, right| left.session_key.cmp(&right.session_key));

    if attention.is_empty() {
        output.push_str(
            "<p class=\"muted\">未发现上下文刷新：所有会话的相邻请求都保持前缀连续。</p>",
        );
    } else {
        write!(
            output,
            "<h3>上下文刷新警告（{} 个会话）</h3>",
            attention.len()
        )?;
        render_table(output, &attention, true, &common_prefix)?;
    }
    if !normal.is_empty() {
        let include_detail = normal.iter().any(|report| report.events.len() > 1);
        const OPEN_LIMIT: usize = 8;
        if normal.len() <= OPEN_LIMIT {
            write!(output, "<h3>会话明细（{}）</h3>", normal.len())?;
            render_table(output, &normal, include_detail, &common_prefix)?;
        } else {
            write!(
                output,
                "<details class=\"sx-detail\"><summary>查看其余 {} 个会话（未发现上下文刷新）</summary>",
                normal.len()
            )?;
            render_table(output, &normal, include_detail, &common_prefix)?;
            output.push_str("</details>");
        }
    }
    Ok(())
}

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

fn session_label(key: &str, common_prefix: &str) -> String {
    let stripped = key
        .strip_prefix(common_prefix)
        .unwrap_or(key)
        .trim_start_matches(['-', '_', '/']);
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

fn effective_refresh_count(report: &SessionReport) -> usize {
    if report.context_refresh_count > 0 {
        report.context_refresh_count
    } else {
        report.fork_count + report.reorder_count + report.reset_count
    }
}

fn effective_abandoned_prefix_bytes(report: &SessionReport) -> usize {
    if report.context_refresh_count > 0 || report.total_abandoned_prefix_bytes > 0 {
        report.total_abandoned_prefix_bytes
    } else {
        report.total_prefix_cut_bytes
    }
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn render_table(
    output: &mut String,
    reports: &[&SessionReport],
    include_detail: bool,
    common_prefix: &str,
) -> Result<()> {
    output.push_str(
        "<table class=\"pt-table\"><thead><tr><th>会话</th><th>请求</th><th>刷新</th>\
         <th>首次刷新</th><th>事件</th><th>最低前缀复用度</th><th>废弃前缀</th>\
         <th>当前复用率</th><th>逐轮</th>",
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
        let refresh_count = effective_refresh_count(report);
        let first_refresh = first_refresh_pair(report)
            .map(|(previous, next)| format!("{previous} → {next}"))
            .unwrap_or_else(|| "—".to_string());
        let minimum_reuse = report
            .events
            .iter()
            .filter_map(|event| event.prefix_reuse_ratio)
            .reduce(f64::min)
            .map(|value| format!("{:.0}%", value * 100.0))
            .unwrap_or_else(|| "—".to_string());
        write!(
            output,
            "<tr class=\"{row_class}\"><td><code>{session}</code></td><td>{requests}</td>\
             <td>{refresh_count}</td><td>{first_refresh}</td><td>{tags}</td><td>{minimum_reuse}</td>\
             <td>{abandoned} B</td>\
             <td><span class=\"sx-bar\"><i style=\"width:{percent:.0}%\"></i></span>{percent:.0}%</td>\
             <td>{spark}</td>{detail}</tr>",
            row_class = if refresh_count > 0 { "sx-warn" } else { "" },
            session = encode_text(&session_label(&report.session_key, common_prefix)),
            requests = report.request_count,
            tags = event_tags(report),
            percent = ratio(lcp, total) * 100.0,
            spark = reuse_spark(report),
            abandoned = effective_abandoned_prefix_bytes(report),
            detail = if include_detail {
                format!("<td>{}</td>", event_detail(report))
            } else {
                String::new()
            },
        )?;
    }
    output.push_str("</tbody></table>");
    Ok(())
}

pub(super) fn first_refresh_pair(report: &SessionReport) -> Option<(usize, usize)> {
    report
        .events
        .iter()
        .enumerate()
        .filter(|(_, event)| event_is_refresh(event))
        .map(|(index, event)| {
            let previous = if event.previous_sequence == 0 {
                index + 1
            } else {
                event.previous_sequence
            };
            let next = if event.next_sequence == 0 {
                index + 2
            } else {
                event.next_sequence
            };
            (previous, next)
        })
        .min_by_key(|(previous, _)| *previous)
}

fn reuse_spark(report: &SessionReport) -> String {
    if report.events.is_empty() {
        return "—".to_string();
    }
    let mut bars = String::from("<span class=\"sx-spark\">");
    let mut lowest = f64::MAX;
    let mut highest = f64::MIN;
    for event in &report.events {
        let value = event
            .prefix_reuse_ratio
            .unwrap_or_else(|| ratio(event.lcp_bytes, event.next_total_bytes));
        lowest = lowest.min(value);
        highest = highest.max(value);
        let class = if value < 0.5 {
            "sx-low"
        } else if value < 0.75 {
            "sx-mid"
        } else {
            "sx-high"
        };
        write!(
            bars,
            "<i class=\"{class}\" style=\"height:{height:.0}%\"></i>",
            height = (value * 100.0).clamp(8.0, 100.0)
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

fn event_tags(report: &SessionReport) -> String {
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

fn event_detail(report: &SessionReport) -> String {
    let mut detail = String::from(
        "<details><summary>事件</summary><table class=\"pt-table\"><thead><tr>\
         <th>轮次</th><th>类型</th><th>风险</th><th>前一请求</th><th>后一请求</th>\
         <th>结构前缀</th><th>保留前缀</th><th>前缀复用度</th><th>废弃前缀</th><th>重算</th>\
         <th>可恢复稳定</th><th>比较域</th><th>块结构</th></tr></thead><tbody>",
    );
    for (index, event) in report.events.iter().enumerate() {
        let reuse = event
            .prefix_reuse_ratio
            .map(|value| format!("{:.0}%", value * 100.0))
            .unwrap_or_else(|| "—".to_string());
        let previous_sequence = if event.previous_sequence == 0 {
            index + 1
        } else {
            event.previous_sequence
        };
        let next_sequence = if event.next_sequence == 0 {
            index + 2
        } else {
            event.next_sequence
        };
        let risk = if event_is_refresh(event) {
            "<span class=\"sx-tag sx-reset\">刷新</span>"
        } else {
            "—"
        };
        write!(
            detail,
            "<tr><td>{} → {}</td><td>{}</td><td>{}</td><td><code>{}</code></td><td><code>{}</code></td>\
             <td>{} units / {} B</td><td>{} B</td><td>{}</td><td>{} B</td><td>{} B</td><td>{} B</td>\
             <td>{}</td><td><code>{}</code></td></tr>",
            previous_sequence,
            next_sequence,
            event_label(&event.event_type),
            risk,
            encode_text(&short_id(&event.prev_request_id)),
            encode_text(&short_id(&event.next_request_id)),
            event.lcp_units,
            event.lcp_bytes,
            event.retained_prefix_bytes,
            reuse,
            event.abandoned_prefix_bytes,
            event.recomputed_bytes,
            event.stable_after_switch_bytes,
            if event.domain_changed { "已变化" } else { "未变化" },
            encode_text(&event.next_block_runs)
        )
        .expect("writing into String cannot fail");
    }
    detail.push_str("</tbody></table></details>");
    detail
}

fn event_is_refresh(event: &SessionSwitchEvent) -> bool {
    event.prefix_reuse_outlier
        || (event.prefix_reuse_ratio.is_none()
            && !matches!(event.event_type, SessionEventType::Append))
}

fn event_label(event: &SessionEventType) -> &'static str {
    match event {
        SessionEventType::Append => "追加",
        SessionEventType::Fork => "分叉",
        SessionEventType::Reorder => "重排",
        SessionEventType::Reset => "重置",
    }
}
