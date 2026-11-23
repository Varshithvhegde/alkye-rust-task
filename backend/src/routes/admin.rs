
use axum::{
    extract::State,
    http::StatusCode,
    middleware,
    routing::get,
    Extension,
    Json,
    Router,
};
use serde::Serialize;
use sqlx::FromRow;

use crate::{
    middleware::auth::AuthUser,
    state::AppState,
};

#[derive(Debug, Serialize, FromRow)]
struct UserResponse {
    id: String,
    email: String,
    role: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users", get(list_users))
}

async fn list_users(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
) -> Result<Json<Vec<UserResponse>>, StatusCode> {
    if user.role != "admin" {
        return Err(StatusCode::FORBIDDEN);
    }

    let users = sqlx::query_as::<_, UserResponse>(
        r#"
        SELECT id, email, role
        FROM users
        ORDER BY email
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|error| {
        eprintln!("Failed to fetch users: {error}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(users))
}
