//! 本文件渲染会话上下文刷新、逐轮前缀复用度与刷新事件证据。

use std::fmt::Write;

use actrail_kv_artifacts::{SessionEventType, SessionReport, SessionSwitchEvent};
use anyhow::Result;
use html_escape::encode_text;

use crate::locale::{namespace, LocaleModule, LocaleSnapshot};

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
    locale: LocaleSnapshot,
) -> Result<()> {
    let l = locale.create(namespace::REPORT_SESSIONS)?;
    write!(
        output,
        "<h2>{}</h2><style>",
        encode_text(l.get("section_title")?)
    )?;
    output.push_str(SESSION_EVIDENCE_CSS);
    output.push_str("</style>");
    write!(
        output,
        "<p class=\"muted\">{}</p>",
        encode_text(l.get("intro")?)
    )?;

    let mut requests = 0usize;
    let mut lcp_bytes = 0usize;
    let mut next_bytes = 0usize;
    let mut refresh_count = 0usize;
    let mut refresh_sessions = 0usize;
    let mut abandoned_bytes = 0usize;
    for report in reports {
        requests += report.request_count;
        let refreshes = effective_refresh_count(report);
        refresh_count += refreshes;
        refresh_sessions += usize::from(refreshes > 0);
        abandoned_bytes += effective_abandoned_prefix_bytes(report);
        for event in &report.events {
            lcp_bytes += event.lcp_bytes;
            next_bytes += event.next_total_bytes;
        }
    }
    write!(
        output,
        "<div class=\"pt-summary\"><div class=\"pt-metric\"><b>{}</b>{}</div>\
         <div class=\"pt-metric\"><b>{}</b>{}</div>\
         <div class=\"pt-metric\"><b>{}</b>{}</div>\
         <div class=\"pt-metric\"><b>{}</b>{}</div>\
         <div class=\"pt-metric\"><b>{:.1}%</b>{}</div>\
         <div class=\"pt-metric\"><b>{}</b> B</div></div>",
        reports.len(),
        encode_text(l.get("sessions")?),
        requests,
        encode_text(l.get("requests")?),
        refresh_count,
        encode_text(l.get("refresh_warnings")?),
        refresh_sessions,
        encode_text(l.get("refresh_sessions")?),
        ratio(lcp_bytes, next_bytes) * 100.0,
        encode_text(l.get("current_ratio")?),
        abandoned_bytes
    )?;
    write!(
        output,
        "<p class=\"muted\">{}</p>",
        encode_text(l.get("note")?)
    )?;

    let common_prefix = common_key_prefix(reports.iter().map(|report| report.session_key.as_str()));
    let endpoints: std::collections::BTreeSet<&str> = reports
        .iter()
        .map(|report| report.endpoint_key.as_str())
        .collect();
    let models: std::collections::BTreeSet<&str> =
        reports.iter().map(|report| report.model.as_str()).collect();
    write!(
        output,
        "<p class=\"muted\">{} <code>{}</code> · {} <code>{}</code>",
        encode_text(l.get("endpoints")?),
        encode_text(&endpoints.into_iter().collect::<Vec<_>>().join(" / ")),
        encode_text(l.get("models")?),
        encode_text(&models.into_iter().collect::<Vec<_>>().join(" / "))
    )?;
    if !common_prefix.is_empty() {
        write!(
            output,
            " · {}",
            encode_text(&l.format(
                "common_prefix_omitted",
                &[("prefix", common_prefix.clone())],
            )?)
        )?;
    }
    output.push_str("</p>");

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
        write!(
            output,
            "<p class=\"muted\">{}</p>",
            encode_text(l.get("no_refresh")?)
        )?;
    } else {
        write!(
            output,
            "<h3>{}</h3>",
            encode_text(&l.format(
                "warning_heading",
                &[("sessions", attention.len().to_string())],
            )?)
        )?;
        render_table(output, &attention, true, &common_prefix, l)?;
    }
    if !normal.is_empty() {
        let include_detail = normal.iter().any(|report| report.events.len() > 1);
        if normal.len() <= 8 {
            write!(
                output,
                "<h3>{}</h3>",
                encode_text(
                    &l.format("detail_heading", &[("sessions", normal.len().to_string())],)?
                )
            )?;
            render_table(output, &normal, include_detail, &common_prefix, l)?;
        } else {
            write!(
                output,
                "<details class=\"sx-detail\"><summary>{}</summary>",
                encode_text(&l.format(
                    "collapsed_detail_heading",
                    &[("sessions", normal.len().to_string())],
                )?)
            )?;
            render_table(output, &normal, include_detail, &common_prefix, l)?;
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
    l: LocaleModule,
) -> Result<()> {
    write!(
        output,
        "<table class=\"pt-table\"><thead><tr><th>{}</th><th>{}</th><th>{}</th>\
         <th>{}</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th>",
        encode_text(l.get("table_session")?),
        encode_text(l.get("table_requests")?),
        encode_text(l.get("table_refreshes")?),
        encode_text(l.get("table_first_refresh")?),
        encode_text(l.get("table_events")?),
        encode_text(l.get("table_min_reuse")?),
        encode_text(l.get("table_abandoned")?),
        encode_text(l.get("table_current_ratio")?),
        encode_text(l.get("table_trend")?)
    )?;
    if include_detail {
        write!(output, "<th>{}</th>", encode_text(l.get("detail")?))?;
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
            tags = event_tags(report, l)?,
            percent = ratio(lcp, total) * 100.0,
            spark = reuse_spark(report),
            abandoned = effective_abandoned_prefix_bytes(report),
            detail = if include_detail {
                format!("<td>{}</td>", event_detail(report, l)?)
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
        .map(|(index, event)| event_sequences(event, index))
        .min_by_key(|(previous, _)| *previous)
}

fn event_sequences(event: &SessionSwitchEvent, index: usize) -> (usize, usize) {
    (
        if event.previous_sequence == 0 {
            index + 1
        } else {
            event.previous_sequence
        },
        if event.next_sequence == 0 {
            index + 2
        } else {
            event.next_sequence
        },
    )
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

fn event_tags(report: &SessionReport, l: LocaleModule) -> Result<String> {
    let mut tags = String::new();
    for (count, class, key) in [
        (report.append_count, "sx-append", "append"),
        (report.fork_count, "sx-fork", "fork"),
        (report.reorder_count, "sx-reorder", "reorder"),
        (report.reset_count, "sx-reset", "reset"),
    ] {
        if count > 0 {
            write!(
                tags,
                "<span class=\"sx-tag {class}\">{}×{count}</span>",
                encode_text(l.get(key)?)
            )
            .expect("writing into String cannot fail");
        }
    }
    Ok(if tags.is_empty() {
        "—".to_string()
    } else {
        tags
    })
}

fn event_detail(report: &SessionReport, l: LocaleModule) -> Result<String> {
    let mut detail = String::from("<details><summary>");
    detail.push_str(&encode_text(l.get("table_events")?));
    detail.push_str("</summary><table class=\"pt-table\"><thead><tr>");
    for key in [
        "event_round",
        "event_type",
        "event_risk",
        "event_previous",
        "event_next",
        "event_structural_prefix",
        "event_retained_prefix",
        "event_reuse_ratio",
        "event_abandoned",
        "event_recomputed",
        "event_recoverable",
        "event_domain",
        "event_blocks",
    ] {
        write!(detail, "<th>{}</th>", encode_text(l.get(key)?))
            .expect("writing into String cannot fail");
    }
    detail.push_str("</tr></thead><tbody>");
    for (index, event) in report.events.iter().enumerate() {
        let (previous_sequence, next_sequence) = event_sequences(event, index);
        let reuse = event
            .prefix_reuse_ratio
            .map(|value| format!("{:.0}%", value * 100.0))
            .unwrap_or_else(|| "—".to_string());
        write!(
            detail,
            "<tr><td>{} → {}</td><td>{}</td><td>{}</td><td><code>{}</code></td><td><code>{}</code></td>\
             <td>{} units / {} B</td><td>{} B</td><td>{}</td><td>{} B</td><td>{} B</td><td>{} B</td>\
             <td>{}</td><td><code>{}</code></td></tr>",
            previous_sequence,
            next_sequence,
            encode_text(event_label(&event.event_type, l)?),
            if event_is_refresh(event) {
                format!(
                    "<span class=\"sx-tag sx-reset\">{}</span>",
                    encode_text(l.get("event_refresh")?)
                )
            } else {
                "—".to_string()
            },
            encode_text(&short_id(&event.prev_request_id)),
            encode_text(&short_id(&event.next_request_id)),
            event.lcp_units,
            event.lcp_bytes,
            event.retained_prefix_bytes,
            reuse,
            event.abandoned_prefix_bytes,
            event.recomputed_bytes,
            event.stable_after_switch_bytes,
            encode_text(if event.domain_changed {
                l.get("event_changed")?
            } else {
                l.get("event_unchanged")?
            }),
            encode_text(&event.next_block_runs)
        )
        .expect("writing into String cannot fail");
    }
    detail.push_str("</tbody></table></details>");
    Ok(detail)
}

fn event_is_refresh(event: &SessionSwitchEvent) -> bool {
    event.prefix_reuse_outlier
        || (event.prefix_reuse_ratio.is_none()
            && !matches!(event.event_type, SessionEventType::Append))
}

fn event_label(event: &SessionEventType, l: LocaleModule) -> Result<&'static str> {
    l.get(match event {
        SessionEventType::Append => "append",
        SessionEventType::Fork => "fork",
        SessionEventType::Reorder => "reorder",
        SessionEventType::Reset => "reset",
    })
}
