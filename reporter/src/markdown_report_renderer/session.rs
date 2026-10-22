//! 本文件渲染会话上下文刷新、逐请求前缀复用度与事件证据。

use std::{collections::BTreeSet, fmt::Write};

use actrail_kv_artifacts::{SessionEventType, SessionReport, SessionSwitchEvent};
use anyhow::Result;

use super::format::{code, short_id, table_code};

pub(super) fn render(output: &mut String, reports: &[SessionReport]) -> Result<()> {
    output.push_str("## 会话上下文刷新与前缀复用\n\n");
    output.push_str(
        "同一 session 内的相邻请求本应保持 append-only。\
         每轮前缀复用度 = 与上一条请求的公共前缀字节 / 上一条内容性上下文总字节；\
         低于 100% 表示上一条上下文未被完整保留，会废弃原有 KVC 前缀。\n\n",
    );

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
    output.push_str("| 指标 | 数值 |\n|---|---:|\n");
    writeln!(output, "| 会话 | {} |", reports.len())?;
    writeln!(output, "| 请求 | {requests} |")?;
    writeln!(output, "| 上下文刷新警告 | {refresh_count} |")?;
    writeln!(output, "| 发生刷新的会话 | {refresh_sessions} |")?;
    writeln!(
        output,
        "| 当前前缀复用占比 | {:.1}% |",
        ratio(lcp_bytes, next_bytes) * 100.0
    )?;
    writeln!(output, "| 废弃前缀 | {abandoned_bytes} B |\n")?;

    output.push_str(
        "> 上下文压缩、subagent 创建或任务切换都可能触发刷新；\
         这些是 KVC 前缀废弃风险，不自动判定为错误，由用户确认是否接受。\n\n",
    );

    let common_prefix = common_key_prefix(reports.iter().map(|report| report.session_key.as_str()));
    let endpoints: BTreeSet<&str> = reports
        .iter()
        .map(|report| report.endpoint_key.as_str())
        .collect();
    let models: BTreeSet<&str> = reports.iter().map(|report| report.model.as_str()).collect();
    writeln!(
        output,
        "端点：{}。模型：{}。\n",
        endpoints
            .into_iter()
            .map(code)
            .collect::<Vec<_>>()
            .join(" / "),
        models.into_iter().map(code).collect::<Vec<_>>().join(" / ")
    )?;
    if !common_prefix.is_empty() {
        writeln!(
            output,
            "会话列省略共有前缀 {}（完整会话名见 analysis.json）。\n",
            code(&common_prefix)
        )?;
    }

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
        output.push_str("未发现上下文刷新：所有会话的相邻请求都保持前缀连续。\n\n");
    } else {
        writeln!(output, "### 上下文刷新警告（{} 个会话）\n", attention.len())?;
        render_session_table(output, &attention, &common_prefix)?;
    }
    if !normal.is_empty() {
        writeln!(output, "### 会话明细（{}）\n", normal.len())?;
        render_session_table(output, &normal, &common_prefix)?;
    }

    for report in reports {
        render_events(output, report, &common_prefix)?;
    }
    Ok(())
}

fn render_session_table(
    output: &mut String,
    reports: &[&SessionReport],
    common_prefix: &str,
) -> Result<()> {
    output.push_str(
        "| 会话 | 请求 | 刷新 | 首次刷新 | 事件 | 最低前缀复用度 | 废弃前缀 | 当前复用率 | 逐轮 |\n\
         |---|---:|---:|---|---|---:|---:|---:|---|\n",
    );
    for report in reports {
        let lcp: usize = report.events.iter().map(|event| event.lcp_bytes).sum();
        let total: usize = report
            .events
            .iter()
            .map(|event| event.next_total_bytes)
            .sum();
        let first_refresh = first_refresh_pair(report)
            .map(|(previous, next)| format!("{previous} → {next}"))
            .unwrap_or_else(|| "—".to_string());
        let minimum = report
            .events
            .iter()
            .filter_map(|event| event.prefix_reuse_ratio)
            .reduce(f64::min)
            .map(|value| format!("{:.0}%", value * 100.0))
            .unwrap_or_else(|| "—".to_string());
        let trend = report
            .events
            .iter()
            .map(|event| {
                let value = event
                    .prefix_reuse_ratio
                    .unwrap_or_else(|| ratio(event.lcp_bytes, event.next_total_bytes));
                format!("{:.0}%", value * 100.0)
            })
            .collect::<Vec<_>>()
            .join(" → ");
        writeln!(
            output,
            "| {} | {} | {} | {} | {} | {} | {} B | {:.0}% | {} |",
            table_code(&session_label(&report.session_key, common_prefix)),
            report.request_count,
            effective_refresh_count(report),
            first_refresh,
            event_tags(report),
            minimum,
            effective_abandoned_prefix_bytes(report),
            ratio(lcp, total) * 100.0,
            if trend.is_empty() { "—" } else { &trend }
        )?;
    }
    output.push('\n');
    Ok(())
}

fn render_events(output: &mut String, report: &SessionReport, common_prefix: &str) -> Result<()> {
    if report.events.is_empty() {
        return Ok(());
    }
    writeln!(
        output,
        "#### 会话事件：{}\n",
        session_label(&report.session_key, common_prefix)
    )?;
    output.push_str(
        "| 轮次 | 类型 | 风险 | 前一请求 | 后一请求 | 结构前缀 | 保留前缀 | 前缀复用度 | 废弃前缀 | 重算 | 可恢复稳定 | 比较域 | 块结构 |\n\
         |---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|---|\n",
    );
    for (index, event) in report.events.iter().enumerate() {
        let (previous, next) = event_sequences(event, index);
        let reuse = event
            .prefix_reuse_ratio
            .map(|value| format!("{:.0}%", value * 100.0))
            .unwrap_or_else(|| "—".to_string());
        writeln!(
            output,
            "| {previous} → {next} | {} | {} | {} | {} | {} units / {} B | {} B | {} | {} B | {} B | {} B | {} | {} |",
            event_label(&event.event_type),
            if event_is_refresh(event) { "刷新" } else { "—" },
            table_code(&short_id(&event.prev_request_id)),
            table_code(&short_id(&event.next_request_id)),
            event.lcp_units,
            event.lcp_bytes,
            event.retained_prefix_bytes,
            reuse,
            event.abandoned_prefix_bytes,
            event.recomputed_bytes,
            event.stable_after_switch_bytes,
            if event.domain_changed { "已变化" } else { "未变化" },
            table_code(&event.next_block_runs)
        )?;
    }
    output.push('\n');
    Ok(())
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

fn first_refresh_pair(report: &SessionReport) -> Option<(usize, usize)> {
    report
        .events
        .iter()
        .enumerate()
        .filter(|(_, event)| event_is_refresh(event))
        .map(|(index, event)| event_sequences(event, index))
        .min_by_key(|(previous, _)| *previous)
}

fn event_is_refresh(event: &SessionSwitchEvent) -> bool {
    event.prefix_reuse_outlier
        || (event.prefix_reuse_ratio.is_none()
            && !matches!(event.event_type, SessionEventType::Append))
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

fn event_tags(report: &SessionReport) -> String {
    let labels = [
        (report.append_count, "追加"),
        (report.fork_count, "分叉"),
        (report.reorder_count, "重排"),
        (report.reset_count, "重置"),
    ]
    .into_iter()
    .filter(|(count, _)| *count > 0)
    .map(|(count, label)| format!("{label}×{count}"))
    .collect::<Vec<_>>();
    if labels.is_empty() {
        "—".to_string()
    } else {
        labels.join("、")
    }
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

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn event_label(event: &SessionEventType) -> &'static str {
    match event {
        SessionEventType::Append => "追加",
        SessionEventType::Fork => "分叉",
        SessionEventType::Reorder => "重排",
        SessionEventType::Reset => "重置",
    }
}
