//! 本文件定义 Session 时间线分析内部使用的 observation 与边界模型。

use actrail_kv_artifacts::SessionBoundaryContext;

use crate::model::{corpus::CorpusRecord, projection::ComparisonDomain};

#[derive(Clone, Debug)]
pub struct SessionObservation {
    pub session_id: String,
    pub request_id: String,
    pub captured_at: Option<String>,
    pub input_line: usize,
    pub source: Option<String>,
    pub sequence_index: Option<usize>,
}

impl SessionObservation {
    pub fn from_record(record: &CorpusRecord, sequence_index: Option<usize>) -> Option<Self> {
        Some(Self {
            session_id: record.session_id.clone()?,
            request_id: record.id.clone(),
            captured_at: record.captured_at.clone(),
            input_line: record.input_line,
            source: record.source.clone(),
            sequence_index,
        })
    }
}

pub fn same_boundary(left: &ComparisonDomain, right: &ComparisonDomain) -> bool {
    left.endpoint_key == right.endpoint_key
        && left.model == right.model
        && left.model_deployment_key == right.model_deployment_key
        && left.context_schema_key == right.context_schema_key
        && left.agent_key == right.agent_key
        && left.kv_namespace == right.kv_namespace
        && left.dialect == right.dialect
        && left.adapter_revision == right.adapter_revision
}

pub fn boundary(domain: &ComparisonDomain) -> SessionBoundaryContext {
    SessionBoundaryContext {
        endpoint_key: domain.endpoint_key.clone(),
        model: domain.model.clone(),
        context_schema_key: domain.context_schema_key.clone(),
        agent_key: domain.agent_key.clone(),
        model_deployment_key: domain.model_deployment_key.clone(),
        kv_namespace: domain.kv_namespace.clone(),
        dialect: domain.dialect.clone(),
        adapter_revision: domain.adapter_revision.clone(),
    }
}
