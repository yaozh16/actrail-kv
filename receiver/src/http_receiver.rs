//! 本文件定义 POST /requests 的输入校验、采集元数据补全和状态码契约。

use std::sync::Arc;

use actrail_kv_artifacts::{CapturedRequest, ComparisonMetadata};
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use serde_json::Value;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::config::DEFAULT_MAX_PAYLOAD_BYTES;
use crate::NdjsonAppender;

const SOURCE_HEADER: &str = "x-actrail-source";
const SESSION_KEY_HEADER: &str = "x-actrail-session-key";
const ENDPOINT_KEY_HEADER: &str = "x-actrail-endpoint-key";
const AGENT_KEY_HEADER: &str = "x-actrail-agent-key";
const MODEL_DEPLOYMENT_KEY_HEADER: &str = "x-actrail-model-deployment-key";
const KV_NAMESPACE_HEADER: &str = "x-actrail-kv-namespace";

pub fn router(appender: Arc<NdjsonAppender>) -> Router {
    router_with_limit(appender, DEFAULT_MAX_PAYLOAD_BYTES)
}

/// 使用自定义请求体上限构建路由，供配置文件覆盖默认 8 MiB 限制。
pub fn router_with_limit(appender: Arc<NdjsonAppender>, max_payload_bytes: usize) -> Router {
    Router::new()
        .route("/requests", post(receive_request))
        .layer(DefaultBodyLimit::max(max_payload_bytes))
        .with_state(appender)
}

async fn receive_request(
    State(appender): State<Arc<NdjsonAppender>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let payload: Value = match serde_json::from_slice(&body) {
        Ok(Value::Object(object)) => Value::Object(object),
        Ok(_) | Err(_) => return client_error("body must be one valid JSON object"),
    };
    let source = match headers.get(SOURCE_HEADER) {
        Some(value) => match value.to_str() {
            Ok(value) => Some(value.to_owned()),
            Err(_) => return client_error("X-Actrail-Source must be valid header text"),
        },
        None => None,
    };
    let session_key = match metadata_header(&headers, SESSION_KEY_HEADER, false) {
        Ok(value) => value,
        Err(message) => return client_error(message),
    };
    let endpoint_key = match metadata_header(&headers, ENDPOINT_KEY_HEADER, true) {
        Ok(Some(value)) => value,
        Ok(None) => return client_error("X-Actrail-Endpoint-Key is required"),
        Err(message) => return client_error(message),
    };
    let agent_key = match metadata_header(&headers, AGENT_KEY_HEADER, false) {
        Ok(value) => value,
        Err(message) => return client_error(message),
    };
    let model_deployment_key = match metadata_header(&headers, MODEL_DEPLOYMENT_KEY_HEADER, false) {
        Ok(value) => value,
        Err(message) => return client_error(message),
    };
    let kv_namespace = match metadata_header(&headers, KV_NAMESPACE_HEADER, false) {
        Ok(value) => value,
        Err(message) => return client_error(message),
    };
    let captured_at = match OffsetDateTime::now_utc().format(&Rfc3339) {
        Ok(value) => value,
        Err(_) => return server_error(),
    };
    let record = CapturedRequest {
        captured_at,
        source,
        session_key,
        comparison: ComparisonMetadata {
            endpoint_key,
            agent_key,
            model_deployment_key,
            kv_namespace,
        },
        payload,
    };

    match tokio::task::spawn_blocking(move || appender.append(&record)).await {
        Ok(Ok(())) => StatusCode::ACCEPTED.into_response(),
        Ok(Err(error)) => {
            eprintln!("failed to append captured request: {error}");
            server_error()
        }
        Err(error) => {
            eprintln!("captured request writer task failed: {error}");
            server_error()
        }
    }
}

fn metadata_header(
    headers: &HeaderMap,
    name: &'static str,
    required: bool,
) -> Result<Option<String>, &'static str> {
    let Some(value) = headers.get(name) else {
        return if required {
            Err("X-Actrail-Endpoint-Key is required")
        } else {
            Ok(None)
        };
    };
    let value = value
        .to_str()
        .map_err(|_| "comparison metadata headers must contain valid text")?
        .trim();
    if value.is_empty() {
        return Err("comparison metadata headers must not be empty");
    }
    Ok(Some(value.to_owned()))
}

fn client_error(message: &'static str) -> Response {
    (StatusCode::BAD_REQUEST, message).into_response()
}

fn server_error() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "failed to persist captured request",
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use std::{fs, sync::Arc};

    use actrail_kv_artifacts::CapturedRequest;
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
    };
    use tempfile::tempdir;
    use tower::ServiceExt;

    use crate::config::DEFAULT_MAX_PAYLOAD_BYTES as MAX_PAYLOAD_BYTES;
    use crate::{router, NdjsonAppender};

    #[tokio::test]
    async fn accepts_object_and_preserves_payload_with_source() {
        let directory = tempdir().expect("create temporary directory");
        let output = directory.path().join("requests.ndjson");
        let app = router(Arc::new(
            NdjsonAppender::open(&output).expect("open appender"),
        ));
        let response = app
            .oneshot(
                Request::post("/requests")
                    .header("content-type", "application/json")
                    .header("X-Actrail-Source", "agent-hook")
                    .header("X-Actrail-Session-Key", "session-001")
                    .header("X-Actrail-Endpoint-Key", "llm-primary")
                    .header("X-Actrail-Agent-Key", "coding-agent")
                    .header("X-Actrail-Model-Deployment-Key", "deployment-a")
                    .header("X-Actrail-KV-Namespace", "cache-scope-a")
                    .body(Body::from(
                        r#"{"model":"example","messages":[{"content":"你好"}]}"#,
                    ))
                    .expect("build request"),
            )
            .await
            .expect("serve request");

        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let line = fs::read_to_string(output).expect("read output");
        assert!(line.ends_with('\n'));
        assert_eq!(line.lines().count(), 1);
        let captured: CapturedRequest = serde_json::from_str(line.trim_end()).expect("parse line");
        assert_eq!(captured.source.as_deref(), Some("agent-hook"));
        assert_eq!(captured.session_key.as_deref(), Some("session-001"));
        assert_eq!(captured.comparison.endpoint_key, "llm-primary");
        assert_eq!(
            captured.comparison.agent_key.as_deref(),
            Some("coding-agent")
        );
        assert_eq!(
            captured.comparison.model_deployment_key.as_deref(),
            Some("deployment-a")
        );
        assert_eq!(
            captured.comparison.kv_namespace.as_deref(),
            Some("cache-scope-a")
        );
        assert_eq!(captured.payload["messages"][0]["content"], "你好");
        assert!(captured.captured_at.ends_with('Z'));
    }

    #[tokio::test]
    async fn requires_explicit_endpoint_key() {
        let directory = tempdir().expect("create temporary directory");
        let output = directory.path().join("requests.ndjson");
        let app = router(Arc::new(
            NdjsonAppender::open(&output).expect("open appender"),
        ));
        let response = app
            .oneshot(
                Request::post("/requests")
                    .body(Body::from(r#"{"model":"example"}"#))
                    .expect("build request"),
            )
            .await
            .expect("serve request");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(fs::metadata(output).expect("output metadata").len(), 0);
    }

    #[tokio::test]
    async fn rejects_malformed_json_and_all_non_object_json() {
        for body in ["{", "null", "[]", "\"text\"", "42", "true"] {
            let directory = tempdir().expect("create temporary directory");
            let output = directory.path().join("requests.ndjson");
            let app = router(Arc::new(
                NdjsonAppender::open(&output).expect("open appender"),
            ));
            let response = app
                .oneshot(
                    Request::post("/requests")
                        .body(Body::from(body))
                        .expect("build request"),
                )
                .await
                .expect("serve request");

            assert_eq!(response.status(), StatusCode::BAD_REQUEST, "body: {body}");
            assert_eq!(fs::metadata(output).expect("output metadata").len(), 0);
        }
    }

    #[tokio::test]
    async fn rejects_body_over_limit_with_payload_too_large() {
        let directory = tempdir().expect("create temporary directory");
        let output = directory.path().join("requests.ndjson");
        let app = router(Arc::new(
            NdjsonAppender::open(&output).expect("open appender"),
        ));
        let body = format!(r#"{{"content":"{}"}}"#, "x".repeat(MAX_PAYLOAD_BYTES));
        let response = app
            .oneshot(
                Request::post("/requests")
                    .body(Body::from(body))
                    .expect("build request"),
            )
            .await
            .expect("serve request");

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(fs::metadata(output).expect("output metadata").len(), 0);
    }

    #[tokio::test]
    async fn returns_method_not_allowed_for_non_post_requests() {
        let directory = tempdir().expect("create temporary directory");
        let app = router(Arc::new(
            NdjsonAppender::open(directory.path().join("requests.ndjson")).expect("open appender"),
        ));

        for method in [Method::GET, Method::PUT, Method::DELETE, Method::PATCH] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri("/requests")
                        .body(Body::empty())
                        .expect("build request"),
                )
                .await
                .expect("serve request");
            assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_http_requests_stay_as_one_ndjson_record_per_line() {
        const REQUESTS: usize = 200;

        let directory = tempdir().expect("create temporary directory");
        let output = directory.path().join("requests.ndjson");
        let app = router(Arc::new(
            NdjsonAppender::open(&output).expect("open appender"),
        ));
        let handles: Vec<_> = (0..REQUESTS)
            .map(|sequence| {
                let app = app.clone();
                tokio::spawn(async move {
                    let response = app
                        .oneshot(
                            Request::post("/requests")
                                .header("X-Actrail-Endpoint-Key", "llm-primary")
                                .body(Body::from(format!(
                                    r#"{{"sequence":{sequence},"content":"line\n{sequence}"}}"#
                                )))
                                .expect("build request"),
                        )
                        .await
                        .expect("serve request");
                    assert_eq!(response.status(), StatusCode::ACCEPTED);
                })
            })
            .collect();

        for handle in handles {
            handle.await.expect("request task succeeds");
        }

        let contents = fs::read_to_string(output).expect("read output");
        let mut seen = vec![false; REQUESTS];
        assert_eq!(contents.lines().count(), REQUESTS);
        for line in contents.lines() {
            let captured: CapturedRequest = serde_json::from_str(line).expect("parse intact line");
            let sequence = captured.payload["sequence"].as_u64().expect("sequence") as usize;
            seen[sequence] = true;
        }
        assert!(seen.into_iter().all(|present| present));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn returns_internal_server_error_when_append_fails() {
        let app = router(Arc::new(
            NdjsonAppender::open("/dev/full").expect("open full device"),
        ));
        let response = app
            .oneshot(
                Request::post("/requests")
                    .header("X-Actrail-Endpoint-Key", "llm-primary")
                    .body(Body::from(r#"{"model":"example"}"#))
                    .expect("build request"),
            )
            .await
            .expect("serve request");

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
