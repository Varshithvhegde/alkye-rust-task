
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json,
    Router,
};

use argon2::{
    password_hash::PasswordHash,
    Argon2,
    PasswordVerifier,
};

use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use jsonwebtoken::{encode, EncodingKey, Header};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::Sha256;
use sqlx::FromRow;
use uuid::Uuid;

use crate::state::{AppState, DevOtp};

type HmacSha256 = Hmac<Sha256>;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct Verify2FARequest {
    pub login_challenge_id: String,
    pub code: String,
}

#[derive(FromRow)]
struct User {
    id: String,
    email: String,
    hashed_password: String,
    role: String,
}

#[derive(FromRow)]
struct LoginChallenge {
    id: String,
    user_id: String,
    code_hash: String,
    expires_at: String,
    consumed_at: Option<String>,
}

#[derive(Serialize)]
struct Claims {
    sub: String,
    email: String,
    role: String,
    exp: usize,
    iat: usize,
    iss: String,
}

fn hash_otp(
    code: &str,
    secret: &str,
) -> Result<String, String> {
    let mut mac = HmacSha256::new_from_slice(
        secret.as_bytes(),
    )
    .map_err(|_| "Invalid OTP secret".to_string())?;

    mac.update(code.as_bytes());

    Ok(hex::encode(mac.finalize().into_bytes()))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/verify-2fa", post(verify_2fa))
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Find user by email.
    let user = sqlx::query_as::<_, User>(
        "SELECT id, email, hashed_password, role
         FROM users
         WHERE email = ?",
    )
    .bind(&payload.email)
    .fetch_optional(&state.db)
    .await
    .map_err(internal_error)?;

    let Some(user) = user else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid email or password".to_string(),
        ));
    };

    // Verify password using Argon2.
    let password_hash = PasswordHash::new(
        &user.hashed_password,
    )
    .map_err(|_| internal_error("Invalid password hash"))?;

    Argon2::default()
        .verify_password(
            payload.password.as_bytes(),
            &password_hash,
        )
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                "Invalid email or password".to_string(),
            )
        })?;

    // Generate a 6-digit OTP.
    let code = format!(
        "{:06}",
        thread_rng().gen_range(0..1_000_000)
    );

    let code_hash = hash_otp(
        &code,
        &state.otp_hash_secret,
    )
    .map_err(internal_error)?;

    let challenge_id = Uuid::new_v4().to_string();

    let expires_at = Utc::now() + Duration::minutes(5);

    // Save challenge.
    sqlx::query(
        "INSERT INTO login_challenges
         (id, user_id, code_hash, expires_at)
         VALUES (?, ?, ?, ?)",
    )
    .bind(&challenge_id)
    .bind(&user.id)
    .bind(&code_hash)
    .bind(expires_at.to_rfc3339())
    .execute(&state.db)
    .await
    .map_err(internal_error)?;

    // Save email event (without plaintext OTP).
    sqlx::query(
        "INSERT INTO email_logs
         (id, user_id, email, subject)
         VALUES (?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&user.id)
    .bind(&user.email)
    .bind("Your login verification code")
    .execute(&state.db)
    .await
    .map_err(internal_error)?;

    // Store OTP in memory for local development only.
    {
        let mut latest = state
            .latest_otp
            .write()
            .map_err(|_| internal_error("OTP lock error"))?;

        *latest = Some(DevOtp {
            email: user.email.clone(),
            code: code.clone(),
        });
    }

    // Print OTP to backend console.
    println!(
        "\n[DEV EMAIL] To: {}\nVerification code: {}\n",
        user.email,
        code
    );

    // IMPORTANT: No JWT is returned at this stage.
    Ok((
        StatusCode::OK,
        Json(json!({
            "message": "Verification code sent",
            "login_challenge_id": challenge_id,
            "expires_in": 300
        })),
    ))
}

async fn verify_2fa(
    State(state): State<AppState>,
    Json(payload): Json<Verify2FARequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Find challenge.
    let challenge = sqlx::query_as::<_, LoginChallenge>(
        "SELECT id, user_id, code_hash, expires_at,
                consumed_at
         FROM login_challenges
         WHERE id = ?",
    )
    .bind(&payload.login_challenge_id)
    .fetch_optional(&state.db)
    .await
    .map_err(internal_error)?;

    let Some(challenge) = challenge else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid login challenge".to_string(),
        ));
    };

    // Reject already-used challenge.
    if challenge.consumed_at.is_some() {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Verification code already used".to_string(),
        ));
    }

    // Check expiry.
    let expires_at = chrono::DateTime::parse_from_rfc3339(
        &challenge.expires_at,
    )
    .map_err(internal_error)?;

    if Utc::now() > expires_at {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Verification code expired".to_string(),
        ));
    }

    // Verify OTP hash.
    let submitted_hash = hash_otp(
        &payload.code,
        &state.otp_hash_secret,
    )
    .map_err(internal_error)?;

    if submitted_hash != challenge.code_hash {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid verification code".to_string(),
        ));
    }

    // Atomically consume challenge.
    let result = sqlx::query(
        "UPDATE login_challenges
         SET consumed_at = ?
         WHERE id = ?
           AND consumed_at IS NULL
           AND expires_at > ?",
    )
    .bind(Utc::now().to_rfc3339())
    .bind(&challenge.id)
    .bind(Utc::now().to_rfc3339())
    .execute(&state.db)
    .await
    .map_err(internal_error)?;

    if result.rows_affected() != 1 {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Challenge expired or already used".to_string(),
        ));
    }

    // Get user details.
    let user = sqlx::query_as::<_, User>(
        "SELECT id, email, hashed_password, role
         FROM users
         WHERE id = ?",
    )
    .bind(&challenge.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(internal_error)?;

    let now = Utc::now();

let claims = Claims {
    sub: user.id.clone(),
    email: user.email.clone(),
    role: user.role.clone(),
    iat: now.timestamp() as usize,
    exp: (now + Duration::minutes(60))
        .timestamp() as usize,
    iss: "task-management-api".to_string(),
};

    // Generate JWT only after successful 2FA.
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(
            state.jwt_secret.as_bytes(),
        ),
    )
    .map_err(internal_error)?;

Ok(Json(json!({
    "message": "Login successful",
    "access_token": token,
    "token_type": "Bearer",
    "expires_in": 3600,
    "user": {
        "id": user.id.clone(),
        "email": user.email.clone(),
        "role": user.role.clone()
    }
})))
}

fn internal_error<E: std::fmt::Display>(
    error: E,
) -> (StatusCode, String) {
    tracing::error!("Authentication error: {}", error);

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Internal server error".to_string(),
    )
}
