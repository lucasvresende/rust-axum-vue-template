use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(serde::Serialize, utoipa::ToSchema)]
pub(crate) struct ErrorResponse {
    detail: String,
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum ApiError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("bad request: {0}")]
    Bad(&'static str),
    #[error("email already registered")]
    Conflict,
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let code = match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Bad(_) => StatusCode::BAD_REQUEST,
            Self::Conflict => StatusCode::CONFLICT,
            Self::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let detail = match &self {
            Self::Db(error) => {
                tracing::error!(%error, "database request failed");
                "internal server error".to_owned()
            }
            _ => self.to_string(),
        };
        (code, Json(ErrorResponse { detail })).into_response()
    }
}

pub(crate) type Result<T> = std::result::Result<T, ApiError>;
