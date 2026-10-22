//! 本文件渲染 Top K 与全部 P1/X/P2 缺陷详情。

use std::{collections::BTreeMap, fmt::Write};

use actrail_kv_artifacts::{AnalysisResult, ContextDefect, DefectFactKind, MismatchPattern};
use anyhow::{anyhow, Result};

use super::format::{code, heading_anchor, inline, table_code};

pub(super) fn render_top_k(output: &mut String, result: &AnalysisResult) -> Result<()> {
    let defects: BTreeMap<_, _> = result
        .defects
        .iter()
        .map(|defect| (defect.id.as_str(), defect))
        .collect();
    output.push_str("## Top K\n\n");
    output.push_str(
        "按 `top_k[]` 的得分序排列（score → 受影响请求数 → 被阻断稳定字节 → 缺陷 ID）；\
         每行给出位置、被阻断稳定字节与建议动作，可跳到下面的证据。\
         被阻断稳定字节是结构代理，不代表真实 Token、KV miss 或金额收益。\n\n",
    );
    if result.top_k.is_empty() {
        output.push_str("本次未发现上下文结构缺陷。\n\n");
        return Ok(());
    }
    output.push_str(
        "| 排名 | 位置 | 被阻断稳定字节 | 受影响请求 | 得分 | 建议动作 | 缺陷 |\n\
         |---:|---|---:|---:|---:|---|---|\n",
    );
    for (index, id) in result.top_k.iter().enumerate() {
        let defect = defects
            .get(id.as_str())
            .ok_or_else(|| anyhow!("top_k references unknown defect {id}"))?;
        let advice = defect
            .insights
            .first()
            .map(|insight| insight.summary.as_str())
            .unwrap_or("—");
        let anchor = defect_anchor(id);
        writeln!(
            output,
            "| {} | {} | {} | {} | {:.2} | {} | [{}](#{}) |",
            index + 1,
            table_code(&defect_location(defect)),
            defect.blocked_stable_bytes,
            defect.affected_count,
            defect.score.score,
            inline(advice),
            table_code(id),
            anchor
        )?;
    }
    output.push('\n');
    Ok(())
}

pub(super) fn render_details(output: &mut String, result: &AnalysisResult) -> Result<()> {
    output.push_str("## 缺陷详情\n\n");
    for defect in &result.defects {
        render_defect(output, defect)?;
    }
    Ok(())
}

fn render_defect(output: &mut String, defect: &ContextDefect) -> Result<()> {
    writeln!(output, "### 缺陷 {}\n", defect_label(&defect.id))?;
    output.push_str("[↑ 返回 Top K](#top-k)\n\n");
    writeln!(output, "- 原始缺陷 ID：{}\n", code(&defect.id))?;
    writeln!(
        output,
        "- 比较组：window={} endpoint={} model={} schema={}",
        code(&defect.comparison_group.time_window_key),
        code(&defect.comparison_group.endpoint_key),
        code(&defect.comparison_group.model),
        code(&defect.comparison_group.context_schema_key)
    )?;
    if let Some(agent) = &defect.comparison_group.agent_key {
        writeln!(output, "- Agent：{}", code(agent))?;
    }
    if let Some(deployment) = &defect.comparison_group.model_deployment_key {
        writeln!(output, "- 模型部署：{}", code(deployment))?;
    }
    if let Some(namespace) = &defect.comparison_group.kv_namespace {
        writeln!(output, "- KV namespace：{}", code(namespace))?;
    }
    writeln!(
        output,
        "- **P1** 公共前缀 {} bytes → **X** {} → **P2** 恢复稳定 {} bytes（支持 {} 条）\n",
        defect.actual_prefix_bytes,
        pattern_label(&defect.mismatch.pattern),
        defect.recovered_stable.utf8_bytes,
        defect.recovered_stable.support_count
    )?;

    output.push_str("#### X 变体\n\n");
    for variant in &defect.mismatch.variants {
        let evidence = &variant.representative;
        writeln!(
            output,
            "- {} 条请求；代表请求 {}；{} bytes",
            variant.member_request_ids.len(),
            code(&evidence.request_id),
            evidence.utf8_bytes
        )?;
        if !evidence.sources.is_empty() {
            write!(output, "  - 位置：")?;
            for (index, source) in evidence.sources.iter().enumerate() {
                if index > 0 {
                    output.push('；');
                }
                write!(output, "{}", code(&source.json_path))?;
            }
            output.push('\n');
        }
        match &evidence.excerpt {
            Some(excerpt) => writeln!(output, "  - 内容：{}\n", code(excerpt))?,
            None => output.push_str("  - 该变体在此区域缺失内容\n\n"),
        }
    }

    if !defect.mismatch.facts.is_empty() {
        output.push_str("#### 已确认事实\n\n");
        for fact in &defect.mismatch.facts {
            write!(output, "- {}", fact_label(&fact.kind))?;
            if let Some(detail) = &fact.detail {
                write!(output, "：{}", inline(detail))?;
            }
            output.push('\n');
        }
        output.push('\n');
    }

    output.push_str("#### P2 稳定证据\n\n");
    writeln!(output, "{}\n", code(&defect.recovered_stable.excerpt))?;
    output.push_str("| 指标 | 数值 |\n|---|---:|\n");
    writeln!(
        output,
        "| 实际前缀 | {} bytes |",
        defect.actual_prefix_bytes
    )?;
    writeln!(
        output,
        "| 潜在前缀 | {} bytes |",
        defect.potential_prefix_bytes
    )?;
    writeln!(
        output,
        "| 阻断稳定字节 | {} bytes |",
        defect.blocked_stable_bytes
    )?;
    writeln!(output, "| 可比较请求 | {} |", defect.comparable_count)?;
    writeln!(output, "| 影响请求 | {} |", defect.affected_count)?;
    writeln!(output, "| 置信度 | {:.3} |\n", defect.confidence)?;

    if !defect.insights.is_empty() {
        output.push_str("#### 优化启示\n\n");
        for insight in &defect.insights {
            write!(output, "- **{}**", inline(&insight.summary))?;
            if let Some(detail) = &insight.detail {
                write!(output, "：{}", inline(detail))?;
            }
            output.push('\n');
        }
        output.push('\n');
    }
    Ok(())
}

pub(super) fn defect_anchor(id: &str) -> String {
    heading_anchor(&format!("缺陷 {}", defect_label(id)))
}

fn defect_label(id: &str) -> String {
    heading_anchor(id)
}

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
