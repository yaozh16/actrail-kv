//! Projector applies protocol field policy without classifying UUIDs, timestamps, or business fields.

use serde_json::{Map, Value};

use crate::model::corpus::CorpusRecord;

use super::{
    CacheSequence, CacheUnit, CacheUnitKind, ComparisonDomain, ProjectionSkip,
    ProjectionSkipReason, SourceLocation,
};

const DIALECT: &str = "openai-compatible-chat";
const ADAPTER_REVISION: &str = "1";

#[derive(Clone, Debug)]
pub struct ProjectionLimits {
    pub max_units: usize,
    pub max_text_unit_bytes: usize,
}

impl Default for ProjectionLimits {
    fn default() -> Self {
        Self {
            max_units: 512,
            max_text_unit_bytes: 1024 * 1024,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct RequestProjector {
    limits: ProjectionLimits,
}

impl RequestProjector {
    pub fn new(limits: ProjectionLimits) -> Self {
        Self { limits }
    }

    pub fn project(&self, record: &CorpusRecord) -> Result<CacheSequence, ProjectionSkip> {
        self.project_inner(record).map_err(|reason| ProjectionSkip {
            request_id: record.id.clone(),
            reason,
        })
    }

    fn project_inner(&self, record: &CorpusRecord) -> Result<CacheSequence, ProjectionSkipReason> {
        let payload = record
            .payload
            .as_object()
            .ok_or(ProjectionSkipReason::PayloadNotObject)?;
        let model = payload
            .get("model")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or(ProjectionSkipReason::MissingModel)?;
        let messages = payload
            .get("messages")
            .ok_or(ProjectionSkipReason::MissingMessages)?
            .as_array()
            .ok_or(ProjectionSkipReason::MessagesNotArray)?;
        let domain = ComparisonDomain {
            dialect: DIALECT.to_owned(),
            model: model.to_owned(),
            adapter_revision: ADAPTER_REVISION.to_owned(),
        };
        let mut units = Vec::new();

        if let Some(tools) = payload.get("tools").and_then(Value::as_array) {
            push_structural(&mut units, "tools:start", "$.tools");
            for (index, tool) in tools.iter().enumerate() {
                let path = format!("$.tools[{index}]");
                let identity = tool_identity(tool);
                push_unit(
                    &mut units,
                    CacheUnitKind::ToolDefinition,
                    "tool-definition",
                    canonical_json(tool),
                    &path,
                    None,
                    None,
                    identity,
                );
            }
            push_structural(&mut units, "tools:end", "$.tools");
        }

        push_structural(&mut units, "messages:start", "$.messages");
        for (index, message) in messages.iter().enumerate() {
            let object = message
                .as_object()
                .ok_or(ProjectionSkipReason::MessageNotObject { index })?;
            let role = object
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let base = format!("$.messages[{index}]");
            push_unit(
                &mut units,
                CacheUnitKind::Role,
                "message:role",
                role.to_owned(),
                &format!("{base}.role"),
                Some(index),
                Some(role),
                None,
            );
            // This order is adapter policy, independent of HTTP object serialization order.
            for field in [
                "name",
                "content",
                "function_call",
                "tool_calls",
                "tool_call_id",
                "refusal",
            ] {
                if let Some(value) = object.get(field) {
                    self.project_message_field(&mut units, index, role, field, value)?;
                }
            }
        }
        push_structural(&mut units, "messages:end", "$.messages");
        if units.len() > self.limits.max_units {
            return Err(ProjectionSkipReason::UnitBudgetExceeded {
                limit: self.limits.max_units,
            });
        }
        Ok(CacheSequence {
            request_id: record.id.clone(),
            domain,
            units,
            projection_reliability_millis: 1000,
        })
    }

    fn project_message_field(
        &self,
        units: &mut Vec<CacheUnit>,
        message_index: usize,
        role: &str,
        field: &str,
        value: &Value,
    ) -> Result<(), ProjectionSkipReason> {
        let path = format!("$.messages[{message_index}].{field}");
        let content = value
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| canonical_json(value));
        if content.len() > self.limits.max_text_unit_bytes {
            return Err(ProjectionSkipReason::TextBudgetExceeded {
                path,
                bytes: content.len(),
                limit: self.limits.max_text_unit_bytes,
            });
        }
        let kind = match field {
            "tool_calls" | "function_call" => CacheUnitKind::ToolCall,
            "tool_call_id" if role == "tool" => CacheUnitKind::ToolResult,
            _ => CacheUnitKind::VisibleText,
        };
        let identity = if matches!(kind, CacheUnitKind::ToolCall | CacheUnitKind::ToolResult) {
            tool_identity(value)
        } else {
            None
        };
        push_unit(
            units,
            kind,
            &format!("message:{role}:{field}"),
            content,
            &path,
            Some(message_index),
            Some(role),
            identity,
        );
        if !value.is_string() {
            // Canonical JSON is an observable structural proxy, not a byte slice of a source string.
            units
                .last_mut()
                .expect("a projected message field was just appended")
                .source
                .utf8_bytes = None;
        }
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn push_unit(
    units: &mut Vec<CacheUnit>,
    kind: CacheUnitKind,
    alignment_key: &str,
    content: String,
    path: &str,
    message_index: Option<usize>,
    role: Option<&str>,
    tool_identity: Option<String>,
) {
    let byte_range = matches!(kind, CacheUnitKind::VisibleText).then_some(0..content.len());
    units.push(CacheUnit {
        kind,
        alignment_key: alignment_key.to_owned(),
        content,
        source: SourceLocation {
            json_path: path.to_owned(),
            utf8_bytes: byte_range,
            message_index,
            role: role.map(str::to_owned),
        },
        tool_identity,
    });
}

fn push_structural(units: &mut Vec<CacheUnit>, marker: &str, path: &str) {
    push_unit(
        units,
        CacheUnitKind::Structural,
        marker,
        marker.to_owned(),
        path,
        None,
        None,
        None,
    );
}

fn tool_identity(value: &Value) -> Option<String> {
    let object = value.as_object()?;
    object
        .get("name")
        .and_then(Value::as_str)
        .or_else(|| {
            object
                .get("function")
                .and_then(Value::as_object)
                .and_then(|function| function.get("name"))
                .and_then(Value::as_str)
        })
        .map(str::to_owned)
}

fn canonical_json(value: &Value) -> String {
    match value {
        Value::Object(object) => canonical_object(object),
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",")
        ),
        _ => serde_json::to_string(value).expect("parsed JSON is serializable"),
    }
}

fn canonical_object(object: &Map<String, Value>) -> String {
    let mut keys: Vec<_> = object.keys().collect();
    keys.sort_unstable();
    let entries = keys.into_iter().map(|key| {
        format!(
            "{}:{}",
            serde_json::to_string(key).expect("JSON key is serializable"),
            canonical_json(&object[key])
        )
    });
    format!("{{{}}}", entries.collect::<Vec<_>>().join(","))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn record(payload: Value) -> CorpusRecord {
        CorpusRecord {
            id: "request".into(),
            captured_at: None,
            source: None,
            payload,
            input_line: 1,
        }
    }

    #[test]
    fn ignores_root_key_order_but_preserves_message_and_tool_order() {
        let payload = json!({
            "temperature": 0.9,
            "messages": [{"content":"你好🌍", "role":"system"}, {"role":"user","content":"go"}],
            "tools": [
                {"type":"function", "function":{"name":"zeta", "description":"z"}},
                {"function":{"description":"a", "name":"alpha"}, "type":"function"}
            ],
            "model":"m"
        });
        let projected = RequestProjector::default()
            .project(&record(payload))
            .unwrap();
        let tool_ids: Vec<_> = projected
            .units
            .iter()
            .filter_map(|unit| unit.tool_identity.as_deref())
            .collect();
        assert_eq!(tool_ids, ["zeta", "alpha"]);
        let unicode = projected
            .units
            .iter()
            .find(|unit| unit.content == "你好🌍")
            .unwrap();
        assert_eq!(unicode.source.utf8_bytes, Some(0..10));
        assert!(unicode.content.is_char_boundary(10));
    }

    #[test]
    fn isolates_models_and_ignores_non_context_fields() {
        let a = RequestProjector::default()
            .project(&record(
                json!({"model":"a", "temperature":0, "messages":[]}),
            ))
            .unwrap();
        let b = RequestProjector::default()
            .project(&record(
                json!({"messages":[], "temperature":1, "model":"b"}),
            ))
            .unwrap();
        assert_ne!(a.domain, b.domain);
        assert_eq!(a.units, b.units);
    }

    #[test]
    fn rejects_at_text_budget_plus_one_without_truncation() {
        let projector = RequestProjector::new(ProjectionLimits {
            max_units: 512,
            max_text_unit_bytes: 4,
        });
        let failure = projector
            .project(&record(
                json!({"model":"m", "messages":[{"role":"user", "content":"12345"}]}),
            ))
            .unwrap_err();
        assert!(matches!(
            failure.reason,
            ProjectionSkipReason::TextBudgetExceeded { .. }
        ));
    }
}
