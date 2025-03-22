use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::fs;
use std::path::Path;

#[async_trait]
pub trait Storage {
    async fn save_event(&self, detail: String) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    pub async fn new(path: &str) -> anyhow::Result<Self> {
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent)?;
        }

        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS events (id INTEGER PRIMARY KEY, detail TEXT NOT NULL)",
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl Storage for SqliteStorage {
    async fn save_event(&self, detail: String) -> anyhow::Result<()> {
        sqlx::query("INSERT INTO events (detail) VALUES (?)")
            .bind(detail)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
