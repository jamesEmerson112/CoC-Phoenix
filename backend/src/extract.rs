//! Extractors that fail through [`AppError`].
//!
//! axum's own `Json` and `Path` reject a bad request *before* the handler body runs, and their
//! rejections render as `text/plain`. That would break the documented contract that errors are
//! always `{"error": {"code", "message", "details": []}}` for exactly the four cases a client
//! hits first: an unknown field, malformed JSON, a missing `content-type`, and a bad UUID in the
//! path. These thin wrappers convert the rejection into an [`AppError`] so the envelope holds
//! everywhere, while keeping the status code axum chose.

use crate::error::AppError;
use axum::async_trait;
use axum::extract::{FromRequest, FromRequestParts, Path, Request};
use axum::http::request::Parts;
use axum::Json;
use serde::de::DeserializeOwned;

/// `Json<T>`, but rejections come back as [`AppError`].
#[derive(Debug, Clone, Copy, Default)]
pub struct AppJson<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for AppJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(AppJson(value)),
            Err(rejection) => Err(AppError::rejection(
                rejection.status(),
                rejection.body_text(),
            )),
        }
    }
}

/// `Path<T>`, but rejections come back as [`AppError`].
#[derive(Debug, Clone, Copy, Default)]
pub struct AppPath<T>(pub T);

#[async_trait]
impl<T, S> FromRequestParts<S> for AppPath<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match Path::<T>::from_request_parts(parts, state).await {
            Ok(Path(value)) => Ok(AppPath(value)),
            Err(rejection) => Err(AppError::rejection(
                rejection.status(),
                rejection.body_text(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request as HttpRequest, StatusCode};
    use axum::routing::patch;
    use axum::Router;
    use serde::Deserialize;
    use tower::ServiceExt;
    use uuid::Uuid;

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct Probe {
        #[allow(dead_code)]
        name: String,
    }

    /// A stand-in for `PATCH /characters/:id`: the same extractor pair, no state, no database.
    async fn probe_handler(AppPath(_id): AppPath<Uuid>, AppJson(_body): AppJson<Probe>) -> &'static str {
        "ok"
    }

    async fn call(uri: &str, body: &str, content_type: Option<&str>) -> (StatusCode, serde_json::Value) {
        let app = Router::new().route("/characters/:id", patch(probe_handler));
        let mut builder = HttpRequest::builder().method("PATCH").uri(uri);
        if let Some(value) = content_type {
            builder = builder.header("content-type", value);
        }
        let response = app
            .oneshot(builder.body(Body::from(body.to_string())).unwrap())
            .await
            .unwrap();

        let status = response.status();
        assert_eq!(
            response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok()),
            Some("application/json"),
            "a client that calls res.json() must never be handed text/plain"
        );
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    fn assert_envelope(body: &serde_json::Value) {
        let error = body.get("error").expect("an `error` object");
        assert!(error.get("code").and_then(|c| c.as_str()).is_some(), "{body}");
        assert!(error.get("message").and_then(|m| m.as_str()).is_some(), "{body}");
        assert!(error.get("details").and_then(|d| d.as_array()).is_some(), "{body}");
    }

    const ID: &str = "/characters/2f3c8a1e-4b5d-4a7e-9f10-0c1d2e3f4a5b";

    #[tokio::test]
    async fn a_body_with_an_unknown_field_keeps_the_error_envelope() {
        let (status, body) = call(ID, r#"{"name":"Ada","nmae":"typo"}"#, Some("application/json")).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_envelope(&body);
        assert_eq!(body["error"]["code"], "unprocessable_entity");
    }

    #[tokio::test]
    async fn malformed_json_keeps_the_error_envelope() {
        let (status, body) = call(ID, "{not json", Some("application/json")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_envelope(&body);
    }

    #[tokio::test]
    async fn a_missing_content_type_keeps_the_error_envelope() {
        let (status, body) = call(ID, r#"{"name":"Ada"}"#, None).await;
        assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
        assert_envelope(&body);
        assert_eq!(body["error"]["code"], "unsupported_media_type");
    }

    #[tokio::test]
    async fn a_malformed_uuid_in_the_path_keeps_the_error_envelope() {
        let (status, body) = call(
            "/characters/not-a-uuid",
            r#"{"name":"Ada"}"#,
            Some("application/json"),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_envelope(&body);
    }

    #[tokio::test]
    async fn a_legal_request_still_reaches_the_handler() {
        let app = Router::new().route("/characters/:id", patch(probe_handler));
        let response = app
            .oneshot(
                HttpRequest::builder()
                    .method("PATCH")
                    .uri(ID)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"Ada"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
