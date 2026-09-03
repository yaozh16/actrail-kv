//! 本模块用 comparison group、逻辑边界和最小资格 P2 anchor 生成文案和值无关的身份。

use sha2::{Digest, Sha256};

use crate::model::projection::ComparisonDomain;

use super::episode::scanner::Recovery;

pub(super) fn defect_id(
    domain: &ComparisonDomain,
    logical_shape: &str,
    mismatch_coordinate_id: &str,
    recovery: &Recovery,
) -> String {
    let material = format!(
        "region-fp-v3\0{}\0{}\0{}\0{:?}\0{:?}\0{:?}\0{}\0{}\0{}\0{}\0{}",
        domain.time_window_key,
        domain.endpoint_key,
        domain.model,
        domain.model_deployment_key,
        domain.agent_key,
        domain.kv_namespace,
        domain.context_schema_key,
        logical_shape,
        mismatch_coordinate_id,
        recovery.anchor_chain_digest,
        recovery.local_shape,
    );
    format!(
        "defect-{}",
        &hex::encode(Sha256::digest(material.as_bytes()))[..24]
    )
}

pub(super) fn conditional_site_id(template_id: &str, logical_id: &str) -> String {
    let material = format!("conditional-site-v1\0{template_id}\0{logical_id}");
    format!(
        "conditional-{}",
        &hex::encode(Sha256::digest(material.as_bytes()))[..24]
    )
}

pub(super) fn digest(parts: impl IntoIterator<Item = impl AsRef<[u8]>>) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_ref());
        hasher.update([0]);
    }
    hex::encode(hasher.finalize())
}
