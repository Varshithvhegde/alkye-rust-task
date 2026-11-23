
mod db;
mod state;
mod routes;
mod models;
mod middleware;


use axum::{
    routing::get,
    Router
};
use state::AppState;
use tower::layer;
use tower_http::cors::{Any, CorsLayer};
use axum::http::{header, Method};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();

    let db = db::init_db()
        .await
        .expect("Database initialization failed");

    let jwt_secret = std::env::var("JWT_SECRET")
        .expect("JWT_SECRET must be set");

    let otp_hash_secret = std::env::var("OTP_HASH_SECRET")
        .expect("OTP_HASH_SECRET must be set");

    let state = AppState::new(
        db,
        jwt_secret,
        otp_hash_secret,
    );

let protected_task_routes = routes::tasks::router()
    .route_layer(axum::middleware::from_fn_with_state(
        state.clone(),
        crate::middleware::auth::require_auth,
    ));

    let protected_admin_routes = routes::admin::router()
    .route_layer(axum::middleware::from_fn_with_state(
        state.clone(),
        crate::middleware::auth::require_auth,
    ));


    let cors = CorsLayer::new()
    .allow_origin("http://localhost:5173".parse::<axum::http::HeaderValue>().unwrap())
    .allow_methods([
        Method::GET,
        Method::POST,
        Method::OPTIONS,
    ])
    .allow_headers([
        header::AUTHORIZATION,
        header::CONTENT_TYPE,
    ]);


let app = Router::new()
    .nest("/auth", routes::auth::router())
    .nest("/dev", routes::dev::router())
    .nest("/seed", routes::users::router())
.merge(protected_task_routes)
    .merge(protected_admin_routes) 
    .layer(cors)
    .with_state(state);


    let listener = tokio::net::TcpListener::bind(
        "127.0.0.1:8080",
    )
    .await
    .expect("Failed to bind address");

    println!("Server running at http://127.0.0.1:8080");

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}
