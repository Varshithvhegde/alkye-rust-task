
use std::sync::{Arc, RwLock};
use std::time::Duration;

use moka::future::Cache;
use sqlx::SqlitePool;
use crate::models::task::TaskView;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,

    pub jwt_secret: String,
    pub otp_hash_secret: String,

    pub latest_otp: Arc<RwLock<Option<DevOtp>>>,

    pub task_cache: Cache<String, Vec<TaskView>>,
}

#[derive(Clone)]
pub struct DevOtp {
    pub email: String,
    pub code: String,
}

impl AppState {
    pub fn new(
        db: SqlitePool,
        jwt_secret: String,
        otp_hash_secret: String,
    ) -> Self {
        let task_cache = Cache::builder()
            .time_to_live(Duration::from_secs(300))
            .max_capacity(1000)
            .build();

        Self {
            db,
            jwt_secret,
            otp_hash_secret,
            latest_otp: Arc::new(RwLock::new(None)),
            task_cache,
        }
    }
}
