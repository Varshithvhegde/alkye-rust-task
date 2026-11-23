
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

use jsonwebtoken::{
    decode,
    Algorithm,
    DecodingKey,
    Validation,
};
use serde::Deserialize;

use crate::state::AppState;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: String,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
    email: String,
    role: String,
    exp: usize,
    iat: usize,
    iss: String,
}

pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let header = request
        .headers()
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token = header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let mut validation = Validation::new(Algorithm::HS256);

    validation.set_issuer(&["task-management-api"]);

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(
            state.jwt_secret.as_bytes(),
        ),
        &validation,
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let claims = token_data.claims;

    // Accept only the roles used by this application.
    if claims.role != "admin" && claims.role != "staff" {
        return Err(StatusCode::FORBIDDEN);
    }

    let user = AuthUser {
        id: claims.sub,
        email: claims.email,
        role: claims.role,
    };

    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}
