//! 本模块严格加载并校验 analysis.json 的 schema、引用、排序与数值契约。

mod common;
mod defect;
mod session;

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::BufReader,
    path::Path,
};

use actrail_kv_artifacts::{AnalysisResult, ANALYSIS_SCHEMA_VERSION};
use anyhow::{bail, Context, Result};

use self::{
    common::{is_probability, validate_group},
    defect::{validate_conditional_sites, validate_defect},
    session::validate_session_analysis,
};

pub fn load_result(path: &Path) -> Result<AnalysisResult> {
    let file = File::open(path)
        .with_context(|| format!("failed to open analysis result {}", path.display()))?;
    let result: AnalysisResult = serde_json::from_reader(BufReader::new(file))
        .with_context(|| format!("invalid analysis result {}", path.display()))?;
    validate(&result)?;
    Ok(result)
}

fn validate(result: &AnalysisResult) -> Result<()> {
    if result.run.schema_version != ANALYSIS_SCHEMA_VERSION {
        bail!(
            "unsupported analysis schema version {}",
            result.run.schema_version
        );
    }
    let templates: BTreeMap<_, _> = result
        .templates
        .iter()
        .map(|template| (&template.id, template))
        .collect();
    if templates.len() != result.templates.len() {
        bail!("analysis contains duplicate template IDs");
    }
    for template in &result.templates {
        if template.id.is_empty() {
            bail!("template ID must not be empty");
        }
        validate_group(&template.comparison_group)?;
        if !is_probability(template.cohesion) {
            bail!("template cohesion must be a finite probability");
        }
        let members: BTreeSet<_> = template.member_request_ids.iter().collect();
        if members.len() != template.member_request_ids.len() {
            bail!("template contains duplicate member IDs");
        }
        if !members.contains(&template.medoid_request_id) {
            bail!("template medoid is not one of its members");
        }
        for span in &template.stable_spans {
            common::validate_source(&span.source)?;
        }
        for slot in &template.slots {
            common::validate_source(&slot.source)?;
            if !is_probability(slot.confidence) {
                bail!("template slot confidence must be a finite probability");
            }
        }
    }

    let defects: BTreeMap<_, _> = result
        .defects
        .iter()
        .map(|defect| (&defect.id, defect))
        .collect();
    if defects.len() != result.defects.len() {
        bail!("analysis contains duplicate defect IDs");
    }
    for defect in &result.defects {
        let template = templates
            .get(&defect.template_id)
            .ok_or_else(|| anyhow::anyhow!("defect references an unknown template"))?;
        if template.comparison_group != defect.comparison_group {
            bail!("defect and template comparison groups do not match");
        }
        validate_defect(defect, &template.member_request_ids)?;
    }
    validate_conditional_sites(result, &templates, &defects)?;
    validate_session_analysis(&result.session_analysis)?;

    if result.top_k.len() > result.run.options.top_k {
        bail!("top_k exceeds the configured limit");
    }
    let mut seen = BTreeSet::new();
    for (index, id) in result.top_k.iter().enumerate() {
        if !seen.insert(id) {
            bail!("top_k contains duplicate defect references");
        }
        if !defects.contains_key(id) {
            bail!("top_k references an unknown direct defect");
        }
        if result.defects.get(index).map(|defect| &defect.id) != Some(id) {
            bail!("top_k must be a prefix of the deterministically sorted defects");
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests;
