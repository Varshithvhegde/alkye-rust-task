
use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Json,
    Router,
};

use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHasher,
        SaltString,
    },
    Argon2,
};

use serde_json::{json, Value};
use sqlx::Sqlite;
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users", post(seed_users))
}

async fn seed_users(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    // Generate unique IDs for the users.
    let admin_id = Uuid::new_v4().to_string();
    let james_id = Uuid::new_v4().to_string();

    // Hash passwords using Argon2.
    let admin_password_hash =
        hash_password("Admin@123").map_err(internal_error)?;

    let james_password_hash =
        hash_password("Bond@123").map_err(internal_error)?;

    // Start transaction so both users are seeded together.
    let mut tx = state.db.begin().await.map_err(internal_error)?;

    // Insert Admin user.
    sqlx::query(
        "INSERT OR IGNORE INTO users
         (id, full_name, email, hashed_password, role)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&admin_id)
    .bind("Admin")
    .bind("admin@example.com")
    .bind(&admin_password_hash)
    .bind("admin")
    .execute(&mut *tx)
    .await
    .map_err(internal_error)?;

    // Insert James Bond user.
    sqlx::query(
        "INSERT OR IGNORE INTO users
         (id, full_name, email, hashed_password, role)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&james_id)
    .bind("James Bond")
    .bind("james.bond@example.com")
    .bind(&james_password_hash)
    .bind("staff")
    .execute(&mut *tx)
    .await
    .map_err(internal_error)?;

    // Commit transaction.
    tx.commit().await.map_err(internal_error)?;

    // Return seeded users.
    Ok(Json(json!({
        "message": "Seed users completed",
        "users": [
            {
                "name": "Admin",
                "email": "admin@example.com",
                "password": "Admin@123",
                "role": "admin"
            },
            {
                "name": "James Bond",
                "email": "james.bond@example.com",
                "password": "Bond@123",
                "role": "staff"
            }
        ]
    })))
}

fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);

    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| error.to_string())
}

fn internal_error<E: std::fmt::Display>(
    error: E,
) -> (StatusCode, String) {
    tracing::error!("Seed users error: {}", error);

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Internal server error".to_string(),
    )
}
