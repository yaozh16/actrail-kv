//! 本文件渲染结构复用率、字节构成及结构代理/真实命中对照。

use std::{collections::BTreeMap, fmt::Write};

use actrail_kv_artifacts::{CacheMetricBasis, KvCacheMetric, PrefixReuseSummary};
use anyhow::Result;

use super::format::{code, human_bytes};

const PREFIX_REUSE_TARGET: f64 = 0.9;
const MAX_CROSS_CHECK_ROWS: usize = 8;

pub(super) fn render(
    output: &mut String,
    summary: &PrefixReuseSummary,
    cache_metrics: &[KvCacheMetric],
) -> Result<()> {
    let current = summary.ratio.clamp(0.0, 1.0);
    let potential = summary.potential_ratio.clamp(current, 1.0);
    let target = PREFIX_REUSE_TARGET.clamp(0.0, 1.0);
    output.push_str("## 结构复用率\n\n");
    output.push_str(
        "口径：会话内相邻请求对的公共前缀字节 / 上下文总字节。\
         这是结构代理，不是真实 KV 命中率，也不代表 Token 或金额收益。\n\n",
    );
    output.push_str("| 指标 | 数值 |\n|---|---:|\n");
    writeln!(output, "| 当前 | {:.1}% |", current * 100.0)?;
    writeln!(output, "| 目标 | {:.0}% |", target * 100.0)?;
    writeln!(
        output,
        "| 补完已知结构问题后的上界 | {:.1}% |",
        potential * 100.0
    )?;
    writeln!(
        output,
        "| 已知可恢复 | {:.1} KB |",
        summary.gain_bytes as f64 / 1024.0
    )?;
    output.push('\n');

    output.push_str("### 字节构成\n\n");
    output.push_str("| 构成 | 字节 |\n|---|---:|\n");
    writeln!(
        output,
        "| 已复用 | {} |",
        human_bytes(summary.composition.reusable_bytes)
    )?;
    writeln!(
        output,
        "| 已知可恢复 | {} |",
        human_bytes(summary.composition.recoverable_bytes)
    )?;
    writeln!(
        output,
        "| 每轮新增 | {} |",
        human_bytes(summary.composition.new_content_bytes)
    )?;
    output.push_str("\n> 会话首个请求没有前序，不进入相邻请求对统计。\n\n");

    if current >= target {
        writeln!(
            output,
            "已高于目标 {:.1} 个百分点；覆盖 {} 个会话的 {} 个相邻请求对。\n",
            (current - target) * 100.0,
            summary.sessions,
            summary.request_pairs
        )?;
    } else if potential >= target {
        writeln!(
            output,
            "距目标 {:.1} 个百分点；补完已知结构问题后可越过目标。\
             覆盖 {} 个会话的 {} 个相邻请求对。\n",
            (target - current) * 100.0,
            summary.sessions,
            summary.request_pairs
        )?;
    } else {
        writeln!(
            output,
            "距目标 {:.1} 个百分点；补完已知结构问题后到 {:.1}%，仍差 {:.1} 个百分点。\
             剩余 {:.1}% 的上下文是每轮新增内容，按前缀缓存口径不可复用——\
             要再往上走需要减少每轮新增量，而不是继续调整内容顺序。\
             覆盖 {} 个会话的 {} 个相邻请求对。\n",
            (target - current) * 100.0,
            potential * 100.0,
            (target - potential) * 100.0,
            (1.0 - potential) * 100.0,
            summary.sessions,
            summary.request_pairs
        )?;
    }

    render_cross_check(output, summary, cache_metrics)
}

#[derive(Default)]
struct SessionHitAggregate {
    prompt_tokens: u64,
    cached_tokens: u64,
    reported: usize,
    estimated: usize,
}

fn render_cross_check(
    output: &mut String,
    summary: &PrefixReuseSummary,
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
    let mut rows = Vec::new();
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
    output.push_str("### 结构代理 vs 真实命中\n\n");
    output.push_str(
        "结构复用率按字节口径（公共前缀 / 上下文），真实命中率按 token 口径\
         （cached / prompt）；两者口径不同，只用来判断方向是否一致，不表示两者应当相等。\
         估算口径的请求不参与对比。\n\n",
    );
    let estimated_only = rows.len() - rows.iter().filter(|row| row.2.is_some()).count();
    if estimated_only == rows.len() {
        writeln!(
            output,
            "本次没有采集到上游 usage（{} 个会话全部为估算口径），无法与真实命中率交叉验证。\n",
            rows.len()
        )?;
        return Ok(());
    }
    rows.retain(|row| row.2.is_some());
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
        writeln!(
            output,
            "另有 {} 个会话没有真实 usage，未参与对比。\n",
            estimated_only
        )?;
    }
    if truncated {
        writeln!(
            output,
            "仅列出差异最大的 {} 个会话（共 {} 个可比会话）。\n",
            MAX_CROSS_CHECK_ROWS, total
        )?;
    }
    output.push_str(
        "| 会话 | 结构复用率（字节） | 真实命中率（token） | 差值 | 口径覆盖 |\n\
         |---|---:|---:|---:|---|\n",
    );
    for (session_key, structural, real_hit_rate, reported, estimated) in rows {
        let rate = real_hit_rate.expect("filtered to real hit rates");
        writeln!(
            output,
            "| {} | {:.1}% | {:.1}% | {:+.1}pp | 真实 {} / 估算 {} |",
            code(session_key),
            structural * 100.0,
            rate * 100.0,
            (structural - rate) * 100.0,
            reported,
            estimated
        )?;
    }
    output.push('\n');
    Ok(())
}
