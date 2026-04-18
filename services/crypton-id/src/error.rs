use axum::{
    async_trait,
    extract::{FromRequest, Request, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{de::DeserializeOwned, Serialize};

// ── AppJson<T> — JSON extractor that maps deserialization errors to AppError ──

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
            Ok(Json(v)) => Ok(AppJson(v)),
            Err(e) => match e {
                JsonRejection::MissingJsonContentType(_) =>
                    Err(AppError::bad_request("content_type_must_be_json")),
                JsonRejection::JsonDataError(_) =>
                    Err(AppError::bad_request(format!("invalid_json_body: {e}"))),
                JsonRejection::JsonSyntaxError(_) =>
                    Err(AppError::bad_request(format!("json_syntax_error: {e}"))),
                JsonRejection::BytesRejection(_) =>
                    Err(AppError::bad_request("failed_to_read_body")),
                _ => Err(AppError::bad_request("bad_request")),
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self { success: true, data: Some(data) }
    }
}

#[derive(Serialize)]
pub struct ErrorEnvelope {
    pub success: bool,
    pub error: ApiErrorBody,
}

#[derive(Serialize)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
}

#[derive(Debug)]
pub struct AppError {
    status: StatusCode,
    /// Internal detail — logged but NEVER sent to clients for 5xx errors.
    message: String,
}

impl AppError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self { status: StatusCode::BAD_REQUEST, message: msg.into() }
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self { status: StatusCode::UNAUTHORIZED, message: msg.into() }
    }

    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self { status: StatusCode::FORBIDDEN, message: msg.into() }
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self { status: StatusCode::NOT_FOUND, message: msg.into() }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self { status: StatusCode::INTERNAL_SERVER_ERROR, message: msg.into() }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (code, msg) = if self.status.is_server_error() {
            tracing::error!(detail = %self.message, status = %self.status, "internal error");
            ("internal_server_error".to_string(), "An unexpected internal error occurred.".to_string())
        } else {
            let readable = self.message.replace("_", " ");
            let mut chars = readable.chars();
            let human_msg = match chars.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
            };
            (self.message.clone(), human_msg)
        };

        let env = ErrorEnvelope {
            success: false,
            error: ApiErrorBody { code, message: msg },
        };
        (self.status, Json(env)).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        Self::internal(e.to_string())
    }
}

impl From<redis::RedisError> for AppError {
    fn from(e: redis::RedisError) -> Self {
        Self::internal(e.to_string())
    }
}
