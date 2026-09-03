//! 本文件校验直接缺陷与条件性局部位点的模板引用、证据和计量关系。

use std::collections::{BTreeMap, BTreeSet};

use actrail_kv_artifacts::{
    AnalysisResult, ConditionalLocalSite, ContextDefect, MismatchRegion, RequestTemplate,
};
use anyhow::{bail, Result};

use super::common::{floats_match, is_probability, validate_group, validate_source};

pub(super) fn validate_defect(defect: &ContextDefect, template_members: &[String]) -> Result<()> {
    if defect.id.is_empty() {
        bail!("defect ID must not be empty");
    }
    validate_group(&defect.comparison_group)?;
    let largest_variant = validate_episode(
        &defect.mismatch,
        &defect.recovered_stable.sources,
        defect.recovered_stable.support_count,
        defect.comparable_count,
        template_members,
    )?;
    validate_impact(
        defect.affected_count,
        defect.comparable_count,
        largest_variant,
        defect.blocked_stable_bytes,
        defect.recovered_stable.utf8_bytes,
        defect.confidence,
        defect.insights.is_empty(),
    )?;
    if defect.potential_prefix_bytes < defect.actual_prefix_bytes {
        bail!("potential prefix is shorter than actual prefix");
    }
    if defect.score.blocked_stable_bytes != defect.blocked_stable_bytes
        || defect.score.affected_count != defect.affected_count
        || defect.score.confidence != defect.confidence
    {
        bail!("score factors do not match defect metrics");
    }
    let expected =
        defect.blocked_stable_bytes as f64 * defect.affected_count as f64 * defect.confidence;
    if !floats_match(defect.score.score, expected) {
        bail!("score does not match its declared factors");
    }
    Ok(())
}

pub(super) fn validate_conditional_sites(
    result: &AnalysisResult,
    templates: &BTreeMap<&String, &RequestTemplate>,
    defects: &BTreeMap<&String, &ContextDefect>,
) -> Result<()> {
    let mut ids = BTreeSet::new();
    let direct_templates: BTreeSet<_> = result
        .defects
        .iter()
        .map(|defect| defect.template_id.as_str())
        .collect();
    let mut previous_key: Option<(&str, usize)> = None;
    for site in &result.conditional_local_sites {
        if site.id.is_empty() || !ids.insert(&site.id) || defects.contains_key(&site.id) {
            bail!("conditional local site IDs must be non-empty and globally unique");
        }
        let template = templates
            .get(&site.template_id)
            .ok_or_else(|| anyhow::anyhow!("conditional local site references unknown template"))?;
        if template.comparison_group != site.comparison_group {
            bail!("conditional local site and template groups do not match");
        }
        if !direct_templates.contains(site.template_id.as_str()) {
            bail!("conditional local site requires a direct defect for its template");
        }
        let key = (site.template_id.as_str(), site.episode_index);
        let expected_episode = match previous_key {
            Some((previous_template, previous_episode)) if previous_template == key.0 => {
                previous_episode + 1
            }
            Some((previous_template, _)) if previous_template < key.0 => 2,
            Some(_) => bail!("conditional local sites must be sorted by template ID"),
            None => 2,
        };
        if site.episode_index != expected_episode || site.episode_index > 4 {
            bail!("conditional episode indices must be contiguous from 2 through at most 4");
        }
        previous_key = Some(key);
        validate_conditional_site(site, &template.member_request_ids)?;
    }
    Ok(())
}

fn validate_conditional_site(
    site: &ConditionalLocalSite,
    template_members: &[String],
) -> Result<()> {
    validate_group(&site.comparison_group)?;
    let largest_variant = validate_episode(
        &site.mismatch,
        &site.recovered_stable.sources,
        site.recovered_stable.support_count,
        site.comparable_count,
        template_members,
    )?;
    validate_impact(
        site.affected_count,
        site.comparable_count,
        largest_variant,
        site.blocked_stable_bytes,
        site.recovered_stable.utf8_bytes,
        site.confidence,
        site.insights.is_empty(),
    )
}

fn validate_episode(
    mismatch: &MismatchRegion,
    recovered_sources: &[actrail_kv_artifacts::SourceLocation],
    recovered_support: usize,
    comparable_count: usize,
    template_members: &[String],
) -> Result<usize> {
    if mismatch.variants.len() < 2 {
        bail!("episode mismatch must contain at least two variants");
    }
    let mut variant_ids = BTreeSet::new();
    let mut members = BTreeSet::new();
    let mut largest_variant = 0usize;
    for variant in &mismatch.variants {
        if variant.member_request_ids.is_empty() {
            bail!("episode variant must contain at least one member");
        }
        if !variant_ids.insert(&variant.fingerprint) {
            bail!("episode contains duplicate variant fingerprints");
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
    if !members.is_subset(&template_members) || members.len() != comparable_count {
        bail!("episode members do not match its template and comparable_count");
    }
    if recovered_support != comparable_count || recovered_sources.is_empty() {
        bail!("recovered stable evidence and support are inconsistent");
    }
    for source in recovered_sources {
        validate_source(source)?;
    }
    Ok(largest_variant)
}

fn validate_impact(
    affected_count: usize,
    comparable_count: usize,
    largest_variant: usize,
    blocked_stable_bytes: usize,
    recovered_stable_bytes: usize,
    confidence: f64,
    insights_empty: bool,
) -> Result<()> {
    if affected_count != comparable_count.saturating_sub(largest_variant) {
        bail!("affected_count does not equal comparable count minus largest variant");
    }
    if affected_count == 0 || insights_empty {
        bail!("an episode must affect requests and provide an optimization insight");
    }
    if blocked_stable_bytes > recovered_stable_bytes {
        bail!("blocked bytes exceed recovered stable bytes");
    }
    if !is_probability(confidence) {
        bail!("episode confidence must be a finite probability");
    }
    Ok(())
}
