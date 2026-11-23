
use sqlx::{
    migrate::MigrateDatabase,
    sqlite::{
        SqliteConnectOptions,
        SqliteJournalMode,
        SqlitePoolOptions,
    },
    Sqlite, SqlitePool,
};
use std::time::Duration;

pub async fn init_db() -> Result<SqlitePool, sqlx::Error> {
    // Create the directory for the SQLite database.
    tokio::fs::create_dir_all("data")
        .await
        .expect("Failed to create data directory");

    let options = SqliteConnectOptions::new()
        .filename("data/tasks.db")
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(10))
        .connect_with(options)
        .await?;

    // Run database migrations.
    sqlx::migrate!(".\\migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    println!("SQLite connected and migrations applied");

    Ok(pool)
}
