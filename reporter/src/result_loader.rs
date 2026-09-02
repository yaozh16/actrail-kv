//! 本文件负责严格加载并校验跨二进制 analysis.json 数据契约。

use std::{fs::File, io::BufReader, path::Path};

use actrail_kv_artifacts::AnalysisResult;
use anyhow::{bail, Context, Result};

pub fn load_result(path: &Path) -> Result<AnalysisResult> {
    let file = File::open(path)
        .with_context(|| format!("failed to open analysis result {}", path.display()))?;
    let result: AnalysisResult = serde_json::from_reader(BufReader::new(file))
        .with_context(|| format!("invalid analysis result {}", path.display()))?;
    validate(&result)?;
    Ok(result)
}

fn validate(result: &AnalysisResult) -> Result<()> {
    use std::collections::{BTreeMap, BTreeSet};

    if result.run.schema_version != "0.1.0" {
        bail!(
            "unsupported analysis schema version {}",
            result.run.schema_version
        );
    }
    let template_ids: BTreeSet<_> = result.templates.iter().map(|item| &item.id).collect();
    if template_ids.len() != result.templates.len() {
        bail!("analysis contains duplicate template IDs");
    }
    let findings: BTreeMap<_, _> = result
        .findings
        .iter()
        .map(|finding| (&finding.id, finding))
        .collect();
    if findings.len() != result.findings.len() {
        bail!("analysis contains duplicate finding IDs");
    }
    let mut seen = BTreeSet::new();
    for (index, entry) in result.top_k.iter().enumerate() {
        if entry.rank != index + 1 {
            bail!("top_k ranks must be contiguous and start at one");
        }
        if !seen.insert(&entry.finding_id) {
            bail!("top_k contains duplicate finding references");
        }
        let finding = findings
            .get(&entry.finding_id)
            .ok_or_else(|| anyhow::anyhow!("top_k references an unknown finding"))?;
        if entry.score != finding.score {
            bail!("top_k score does not match referenced finding");
        }
    }
    for finding in &result.findings {
        if !template_ids.contains(&finding.template_id) {
            bail!("finding references an unknown template");
        }
    }
    Ok(())
}
