//! 本文件渲染逐请求 KV 缓存命中率、时延与口径覆盖。

use std::fmt::Write;

use actrail_kv_artifacts::{AnalysisResult, CacheMetricBasis};
use anyhow::Result;

use super::format::table_code;

pub(super) fn render(output: &mut String, result: &AnalysisResult) -> Result<()> {
    output.push_str("## KV 缓存命中率\n\n");
    output.push_str(
        "命中率 = cached_tokens / prompt_tokens（真实 usage）；\
         无 usage 时按与会话内上一条请求的上下文公共前缀估算，并在“口径”列标注。\n\n",
    );
    let reported = result
        .cache_metrics
        .iter()
        .filter(|metric| metric.basis == CacheMetricBasis::Reported)
        .count();
    let estimated = result.cache_metrics.len().saturating_sub(reported);
    let average = result
        .cache_metrics
        .iter()
        .map(|metric| metric.hit_rate)
        .sum::<f64>()
        / result.cache_metrics.len() as f64;
    output.push_str(
        "| 请求 | 平均命中率 | 真实 usage | 估算 |\n\
         |---:|---:|---:|---:|\n",
    );
    writeln!(
        output,
        "| {} | {:.1}% | {} | {} |\n",
        result.cache_metrics.len(),
        average * 100.0,
        reported,
        estimated
    )?;

    output.push_str(
        "| 会话 | 序号 | 请求 | prompt | hit(cached) | miss | output | TTFT | 总耗时 | 命中率 | Δ | 口径 | 明细 |\n\
         |---|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|\n",
    );
    for metric in &result.cache_metrics {
        let session = metric
            .session_key
            .as_deref()
            .map(table_code)
            .unwrap_or_else(|| "（无 session，按输入顺序）".to_string());
        let tokens = |value: Option<u32>| {
            value
                .map(|value| value.to_string())
                .unwrap_or_else(|| "—".to_string())
        };
        let latency = |value: Option<u64>| {
            value
                .map(|value| format!("{value} ms"))
                .unwrap_or_else(|| "—".to_string())
        };
        let delta = metric
            .hit_rate_delta
            .map(|value| {
                format!(
                    "{}{:.1}pp",
                    if value >= 0.0 { "+" } else { "" },
                    value * 100.0
                )
            })
            .unwrap_or_else(|| "—".to_string());
        let detail = match metric.basis {
            CacheMetricBasis::Reported => "—".to_string(),
            CacheMetricBasis::Estimated => format!(
                "LCP {}B / payload {}B",
                metric.estimated_lcp_bytes.unwrap_or(0),
                metric.payload_bytes
            ),
        };
        writeln!(
            output,
            "| {session} | {} | {} | {} | {} | {} | {} | {} | {} | {:.1}% | {} | {} | {} |",
            metric.sequence,
            table_code(&metric.request_id[..metric.request_id.len().min(12)]),
            tokens(metric.prompt_tokens),
            tokens(metric.cached_tokens),
            tokens(metric.miss_tokens),
            tokens(metric.output_tokens),
            latency(metric.ttft_ms),
            latency(metric.total_ms),
            metric.hit_rate * 100.0,
            delta,
            match metric.basis {
                CacheMetricBasis::Reported => "真实",
                CacheMetricBasis::Estimated => "估算",
            },
            detail
        )?;
    }
    output.push('\n');
    Ok(())
}
