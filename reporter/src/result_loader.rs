//! 本文件严格加载并校验统一 P1/X/P2 analysis.json 的引用与数值契约。

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::BufReader,
    path::Path,
};

use actrail_kv_artifacts::{
    AnalysisResult, ContextDefect, SourceLocation, ANALYSIS_SCHEMA_VERSION,
};
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
    if result.top_k.len() > result.run.options.top_k {
        bail!("top_k exceeds the configured limit");
    }
    let mut seen = BTreeSet::new();
    for (index, id) in result.top_k.iter().enumerate() {
        if !seen.insert(id) {
            bail!("top_k contains duplicate defect references");
        }
        if !defects.contains_key(id) {
            bail!("top_k references an unknown defect");
        }
        if result.defects.get(index).map(|defect| &defect.id) != Some(id) {
            bail!("top_k must be a prefix of the deterministically sorted defects");
        }
    }
    Ok(())
}

fn validate_defect(defect: &ContextDefect, template_members: &[String]) -> Result<()> {
    validate_group(&defect.comparison_group)?;
    if defect.mismatch.variants.len() < 2 {
        bail!("defect mismatch must contain at least two variants");
    }
    let mut variant_ids = BTreeSet::new();
    let mut members = BTreeSet::new();
    let mut largest_variant = 0usize;
    for variant in &defect.mismatch.variants {
        if variant.member_request_ids.is_empty() {
            bail!("defect variant must contain at least one member");
        }
        if !variant_ids.insert(&variant.fingerprint) {
            bail!("defect contains duplicate variant fingerprints");
        }
        if !variant
            .member_request_ids
            .iter()
            .any(|id| id == &variant.representative.request_id)
        {
            bail!("variant representative is not a member of its variant");
        }
        for member in &variant.member_request_ids {
            if !members.insert(member) {
                bail!("a comparable member appears in more than one variant");
            }
        }
        largest_variant = largest_variant.max(variant.member_request_ids.len());
        for source in &variant.representative.sources {
            validate_source(source)?;
        }
        if variant.representative.sources.is_empty()
            && (variant.representative.utf8_bytes != 0 || variant.representative.excerpt.is_some())
        {
            bail!("a source-less missing variant must have zero bytes and no excerpt");
        }
    }
    let template_members: BTreeSet<_> = template_members.iter().collect();
    if !members.is_subset(&template_members) {
        bail!("defect variant contains a request outside its template");
    }
    if members.len() != defect.comparable_count {
        bail!("variant members do not match comparable_count");
    }
    if defect.recovered_stable.support_count != defect.comparable_count {
        bail!("recovered stable support does not match comparable_count");
    }
    if defect.recovered_stable.sources.is_empty() {
        bail!("recovered stable evidence must contain a source");
    }
    if defect.affected_count != defect.comparable_count.saturating_sub(largest_variant) {
        bail!("affected_count does not equal comparable count minus largest variant");
    }
    if defect.affected_count == 0 || defect.insights.is_empty() {
        bail!("a defect must affect requests and provide an optimization insight");
    }
    if defect.potential_prefix_bytes < defect.actual_prefix_bytes {
        bail!("potential prefix is shorter than actual prefix");
    }
    if defect.blocked_stable_bytes > defect.recovered_stable.utf8_bytes {
        bail!("blocked bytes exceed recovered stable bytes");
    }
    if !is_probability(defect.confidence)
        || !is_probability(defect.score.confidence)
        || defect.score.confidence != defect.confidence
    {
        bail!("invalid or inconsistent defect confidence");
    }
    if defect.score.blocked_stable_bytes != defect.blocked_stable_bytes
        || defect.score.affected_count != defect.affected_count
    {
        bail!("score factors do not match defect metrics");
    }
    let expected =
        defect.blocked_stable_bytes as f64 * defect.affected_count as f64 * defect.confidence;
    let tolerance = f64::EPSILON * expected.abs().max(1.0) * 8.0;
    if !defect.score.score.is_finite() || (defect.score.score - expected).abs() > tolerance {
        bail!("score does not match its declared factors");
    }
    for source in &defect.recovered_stable.sources {
        validate_source(source)?;
    }
    Ok(())
}

fn validate_source(source: &SourceLocation) -> Result<()> {
    if source.json_path.is_empty() {
        bail!("source JSON path must not be empty");
    }
    match (source.byte_start, source.byte_end) {
        (Some(start), Some(end)) if start <= end => Ok(()),
        (None, None) => Ok(()),
        _ => bail!("source byte offsets must be a valid start/end pair"),
    }
}

fn validate_group(group: &actrail_kv_artifacts::ComparisonGroup) -> Result<()> {
    if group.time_window_key.is_empty()
        || group.endpoint_key.is_empty()
        || group.model.is_empty()
        || group.context_schema_key.is_empty()
    {
        bail!("comparison group required keys must not be empty");
    }
    Ok(())
}

fn is_probability(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

#[cfg(test)]
pub(crate) mod tests {
    use std::{fs, path::Path};

    use actrail_kv_artifacts::*;

    use super::load_result;

    #[test]
    fn accepts_valid_unified_result() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("analysis.json");
        write(&path, &fixture());
        assert!(load_result(&path).is_ok());
    }

    #[test]
    fn rejects_wrong_schema_and_broken_top_k() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("analysis.json");
        let mut result = fixture();
        result.run.schema_version = "legacy".into();
        write(&path, &result);
        assert!(load_result(&path).is_err());

        let mut result = fixture();
        result.top_k = vec!["unknown".into()];
        write(&path, &result);
        assert!(load_result(&path).is_err());
    }

    #[test]
    fn rejects_overlapping_variant_members_and_inconsistent_score() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("analysis.json");
        let mut result = fixture();
        result.defects[0].mismatch.variants[1]
            .member_request_ids
            .push("a".into());
        write(&path, &result);
        assert!(load_result(&path).is_err());

        let mut result = fixture();
        result.defects[0].score.score += 1.0;
        write(&path, &result);
        assert!(load_result(&path).is_err());
    }

    #[test]
    fn accepts_machine_precision_score_rounding() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("analysis.json");
        let mut result = fixture();
        let defect = &mut result.defects[0];
        let expected =
            defect.blocked_stable_bytes as f64 * defect.affected_count as f64 * defect.confidence;
        defect.score.score = expected + f64::EPSILON * expected * 4.0;
        write(&path, &result);
        assert!(load_result(&path).is_ok());
    }

    fn write(path: &Path, result: &AnalysisResult) {
        fs::write(path, serde_json::to_vec(result).expect("serialize")).expect("write fixture");
    }

    pub(crate) fn fixture() -> AnalysisResult {
        let group = ComparisonGroup {
            time_window_key: "window".into(),
            endpoint_key: "endpoint".into(),
            model: "model".into(),
            context_schema_key: "schema".into(),
            agent_key: None,
            model_deployment_key: None,
            kv_namespace: None,
        };
        let source = SourceLocation {
            json_path: "$.messages[0].content".into(),
            logical_scope: vec!["messages".into()],
            unit_index: Some(0),
            byte_start: Some(0),
            byte_end: Some(8),
            role: Some("system".into()),
        };
        let score = ScoreBreakdown {
            blocked_stable_bytes: 100,
            affected_count: 2,
            confidence: 0.9,
            score: 180.0,
        };
        AnalysisResult {
            run: AnalysisRunSummary {
                schema_version: ANALYSIS_SCHEMA_VERSION.into(),
                input_records: 4,
                analyzed_records: 4,
                skipped_records: vec![],
                options: options(),
                limitations: vec![],
            },
            templates: vec![RequestTemplate {
                id: "template".into(),
                comparison_group: group.clone(),
                member_request_ids: vec!["a".into(), "b".into(), "c".into(), "d".into()],
                medoid_request_id: "a".into(),
                cohesion: 0.9,
                stable_spans: vec![],
                slots: vec![],
                prefix_view: None,
            }],
            defects: vec![ContextDefect {
                id: "defect".into(),
                comparison_group: group,
                template_id: "template".into(),
                mismatch: MismatchRegion {
                    pattern: MismatchPattern::ValueMismatch,
                    variants: vec![
                        variant(
                            "one",
                            &["a", "b"],
                            "a",
                            "<script>alert(1)</script>",
                            &source,
                        ),
                        variant("two", &["c", "d"], "c", "south", &source),
                    ],
                    facts: vec![DefectFact {
                        kind: DefectFactKind::ContentVariation,
                        detail: Some("<img src=x onerror=alert(1)>".into()),
                    }],
                },
                recovered_stable: RecoveredStable {
                    sources: vec![source],
                    utf8_bytes: 100,
                    support_count: 4,
                    excerpt: "<script>stable()</script>".into(),
                },
                actual_prefix_bytes: 10,
                potential_prefix_bytes: 110,
                blocked_stable_bytes: 100,
                comparable_count: 4,
                affected_count: 2,
                confidence: 0.9,
                score,
                insights: vec![OptimizationInsight {
                    summary: "<b>统一生成方式</b>".into(),
                    detail: Some("move & review".into()),
                }],
            }],
            top_k: vec!["defect".into()],
            session_reports: vec![],
            cache_metrics: vec![],
        }
    }

    fn variant(
        fingerprint: &str,
        members: &[&str],
        representative: &str,
        excerpt: &str,
        source: &SourceLocation,
    ) -> MismatchVariant {
        MismatchVariant {
            fingerprint: fingerprint.into(),
            member_request_ids: members.iter().map(|member| (*member).into()).collect(),
            representative: VariantEvidence {
                request_id: representative.into(),
                sources: vec![source.clone()],
                utf8_bytes: excerpt.len(),
                excerpt: Some(excerpt.into()),
            },
        }
    }

    fn options() -> AnalysisOptionsSnapshot {
        AnalysisOptionsSnapshot {
            top_k: 1,
            comparison_window_seconds: 3_600,
            min_template_members: 3,
            stable_span_support_ratio: 0.8,
            min_stable_support: 3,
            fixed_variant_max: 3,
            min_blocked_stable_bytes: 64,
            min_exact_anchor_bytes: 24,
            text_similarity_threshold: 0.8,
            template_compatibility_threshold: 0.68,
            max_dynamic_coverage_ratio: 0.35,
            max_candidates_per_request: 128,
            max_projection_units: 512,
            max_text_unit_bytes: 1_048_576,
            max_payload_bytes: 8_388_608,
            max_alignment_cells: 2_000_000,
            max_total_alignment_cells: 64_000_000,
            max_records: 1_000_000,
            prefix_view_max_nodes: 5_000,
            prefix_view_excerpt_bytes: 96,
        }
    }
}
