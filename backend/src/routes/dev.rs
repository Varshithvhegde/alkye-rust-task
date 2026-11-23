
use axum::{
    extract::State,
    http::StatusCode,
    routing::get,
    Json,
    Router,
};

use serde_json::{json, Value};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/email-logs/latest", get(latest_email))
}

async fn latest_email(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let latest = state
        .latest_otp
        .read()
        .map_err(|_| internal_error("OTP lock error"))?;

    let Some(otp) = latest.as_ref() else {
        return Err((
            StatusCode::NOT_FOUND,
            "No development email found".to_string(),
        ));
    };

    Ok(Json(json!({
        "email": otp.email,
        "code": otp.code
    })))
}

fn internal_error<E: std::fmt::Display>(
    error: E,
) -> (StatusCode, String) {
    tracing::error!("Dev email error: {}", error);

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Internal server error".to_string(),
    )
}
