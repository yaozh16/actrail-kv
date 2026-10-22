//! 本文件渲染模板前缀树、成员衰减和结构片段明细。

use std::{collections::BTreeSet, fmt::Write};

use actrail_kv_artifacts::{
    AnalysisResult, PrefixNode, PrefixNodeKind, RequestTemplate, TemplatePrefixView,
};
use anyhow::Result;

use super::{
    defects::defect_anchor,
    format::{code, excerpt, fenced, heading_anchor, inline, short_id, table_code},
};

pub(super) fn render(output: &mut String, result: &AnalysisResult) -> Result<()> {
    let templates: Vec<_> = result
        .templates
        .iter()
        .filter_map(|template| template.prefix_view.as_ref().map(|view| (template, view)))
        .collect();
    if templates.is_empty() {
        return Ok(());
    }
    output.push_str("## 模板前缀树\n\n");
    output.push_str(
        "- **P**：稳定片段，区间内所有成员内容一致，可被后续请求复用。\n\
         - **X**：变体片段，成员在这里分叉。\n\
         - **X（缺陷）**：该分叉阻断了其后的稳定前缀，已被判为结构缺陷。\n\
         - **支持度 3/4**：4 个成员请求中有 3 个包含该片段。\n\
         - 每个模板先给分布与衰减概览，再按顺序列出结构片段。\n\n",
    );

    if templates.len() > 1 {
        output.push_str(
            "| 模板 | 成员 | 稳定字节 | 变体片段 | 缺陷片段 |\n\
             |---|---|---:|---:|---:|\n",
        );
        for (template, view) in &templates {
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
            let anchor = template_anchor(&template.id);
            writeln!(
                output,
                "| [{}](#{}) | {} | {} B | {} | {} |",
                table_code(&excerpt(&template.id, 16)),
                anchor,
                template.member_request_ids.len(),
                stable_bytes,
                dynamic_count,
                defect_count
            )?;
        }
        output.push('\n');
    }

    for (template, view) in templates {
        render_template(output, template, view)?;
    }
    Ok(())
}

fn render_template(
    output: &mut String,
    template: &RequestTemplate,
    view: &TemplatePrefixView,
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
    let by_id: std::collections::HashMap<&str, &PrefixNode> = view
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
    let stable_before_first_x = first_dynamic
        .map(|end| chain[..end].iter().map(|node| node.utf8_bytes).sum())
        .unwrap_or(stable_bytes);
    writeln!(output, "### 模板 {}\n", heading_anchor(&template.id))?;
    writeln!(output, "- 完整 ID：{}\n", code(&template.id))?;
    writeln!(
        output,
        "- {} 个成员请求；结构片段 {} 个（P {} / X {}）。\n",
        member_count,
        view.nodes.len(),
        stable_nodes.len(),
        dynamic_nodes.len()
    )?;
    output.push_str("| 指标 | 数值 |\n|---|---:|\n");
    writeln!(output, "| 稳定字节 | {stable_bytes} B |")?;
    writeln!(output, "| 变体区字节 | {dynamic_bytes} B |")?;
    writeln!(output, "| 变体总数 | {variant_total} |")?;
    writeln!(output, "| 缺失变体 | {missing_total} |")?;
    writeln!(output, "| 缺陷片段 | {defect_nodes} |\n")?;

    let total_bytes = stable_bytes.saturating_add(dynamic_bytes).max(1);
    let stable_pct = stable_bytes as f64 * 100.0 / total_bytes as f64;
    let dynamic_pct = dynamic_bytes as f64 * 100.0 / total_bytes as f64;
    output.push_str("#### 复用分布\n\n");
    output.push_str("| 区域 | 字节 | 占比 |\n|---|---:|---:|\n");
    writeln!(output, "| 稳定前缀 | {stable_bytes} B | {stable_pct:.1}% |")?;
    writeln!(
        output,
        "| 变体/缺失区 | {dynamic_bytes} B | {dynamic_pct:.1}% |\n"
    )?;
    writeln!(
        output,
        "首个分叉点之前累计稳定前缀 **{stable_before_first_x} B**。\n"
    )?;
    if let Some(first_x) = first_dynamic {
        let node = chain[first_x];
        writeln!(
            output,
            "首个分叉点在第 {} 段「{}」，{}/{} 个成员包含该片段（{:.0}%）。\n",
            first_x + 1,
            excerpt(&node.excerpt, 48),
            node.support_count,
            member_count,
            node.support_ratio * 100.0
        )?;
    }

    if chain.len() > 1 {
        output.push_str("#### 成员衰减\n\n");
        output.push_str(
            "| 片段序号 | 类型 | 存活成员数 | 支持率 |\n\
             |---:|---|---:|---:|\n",
        );
        for (index, node) in chain.iter().enumerate() {
            writeln!(
                output,
                "| {} | {} | {}/{} | {:.1}% |",
                index,
                if node.kind == PrefixNodeKind::Dynamic {
                    "X"
                } else {
                    "P"
                },
                node.support_count,
                member_count,
                node.support_ratio * 100.0
            )?;
        }
        writeln!(output, "\n曲线每次下坠代表一批成员从该片段开始分叉。\n")?;
    }

    output.push_str("#### 结构骨架\n\n");
    for (index, node) in chain.iter().enumerate() {
        render_node(output, template, node, index, member_count)?;
    }
    if view.truncated {
        output.push_str("该模板节点数超过上限，视图已截断。\n\n");
    }
    writeln!(output, "[返回模板列表](#模板前缀树)\n")?;
    Ok(())
}

fn render_node(
    output: &mut String,
    template: &RequestTemplate,
    node: &PrefixNode,
    index: usize,
    member_count: usize,
) -> Result<()> {
    let kind = if node.kind == PrefixNodeKind::Dynamic {
        "X"
    } else {
        "P"
    };
    writeln!(
        output,
        "##### 片段 #{index} · {kind} · {}\n",
        inline(&excerpt(&node.excerpt, 120))
    )?;
    writeln!(
        output,
        "- 类型：{kind}{}",
        if node.defect_ids.is_empty() {
            ""
        } else {
            "（缺陷）"
        }
    )?;
    write!(output, "- 来源：{}", code(&node.source.json_path))?;
    if let Some(role) = node.source.role.as_deref() {
        write!(output, "；role={}", code(role))?;
    }
    output.push('\n');
    let range = match (node.source.byte_start, node.source.byte_end) {
        (Some(start), Some(end)) => format!("bytes {start}..{end}"),
        _ => "bytes —".to_string(),
    };
    write!(output, "- 位置：{}", code(&range))?;
    if let Some(unit) = node.source.unit_index {
        write!(output, "；单元 #{unit}")?;
    }
    output.push('\n');
    writeln!(
        output,
        "- 字节数：{} B；支持度：{}/{}（{:.0}%）",
        node.utf8_bytes,
        node.support_count,
        member_count,
        node.support_ratio * 100.0
    )?;
    writeln!(output, "- 摘要：{}\n", code(&node.excerpt))?;

    if let Some(variants) = &node.variants {
        if !variants.is_empty() {
            output.push_str("变体：\n\n");
            output.push_str("| 变体 | 次数 |\n|---|---:|\n");
            let mut ordered = variants.iter().collect::<Vec<_>>();
            ordered.sort_by_key(|variant| std::cmp::Reverse(variant.count));
            for variant in ordered {
                let missing = variant.excerpt.starts_with("（此位置缺失");
                let label = if missing {
                    "此位置缺失".to_string()
                } else {
                    variant.excerpt.clone()
                };
                let mut count = if variant.count_known {
                    variant.count.to_string()
                } else {
                    format!("{}（约）", variant.count)
                };
                if variant.count == 0 {
                    count.push_str("（未出现）");
                }
                writeln!(output, "| {} | {} |", table_code(&label), count)?;
            }
            output.push('\n');
        }
    }

    if !node.defect_ids.is_empty() {
        output.push_str("关联缺陷：\n\n");
        for id in &node.defect_ids {
            writeln!(output, "- [{}](#{})", code(id), defect_anchor(id))?;
        }
        output.push('\n');
    }

    if let Some(sample) = &node.content_sample {
        if !sample.is_empty() {
            output.push_str("内容样本：\n\n");
            writeln!(output, "{}\n", fenced(sample))?;
        }
    }

    let all: BTreeSet<&str> = template
        .member_request_ids
        .iter()
        .map(String::as_str)
        .collect();
    let alive: BTreeSet<&str> = node.member_request_ids.iter().map(String::as_str).collect();
    let lost: Vec<&str> = all.difference(&alive).copied().collect();
    if node.kind == PrefixNodeKind::Dynamic || !lost.is_empty() {
        writeln!(
            output,
            "- 存活 {} / {}：{}",
            alive.len(),
            all.len(),
            member_list(alive.iter().copied())
        )?;
        if !lost.is_empty() {
            writeln!(
                output,
                "- 衰减 {}：{}",
                lost.len(),
                member_list(lost.iter().copied())
            )?;
        }
        output.push('\n');
    }
    Ok(())
}

fn member_list<'a>(ids: impl Iterator<Item = &'a str>) -> String {
    let mut values = Vec::new();
    for (index, id) in ids.enumerate() {
        if index == 8 {
            values.push("…".to_string());
            break;
        }
        values.push(code(&short_id(id)));
    }
    if values.is_empty() {
        "—".to_string()
    } else {
        values.join("、")
    }
}

fn template_anchor(id: &str) -> String {
    heading_anchor(&format!("模板 {}", heading_anchor(id)))
}
