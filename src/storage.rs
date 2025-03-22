use async_trait::async_trait;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[async_trait]
pub trait Storage: Send + Sync {
    async fn save_event(&self, detail: String) -> anyhow::Result<()>;

    async fn record_request(&self, ip: &str, path: &str, key: Option<&str>) -> anyhow::Result<()>;

    async fn is_abusive(&self, ip: &str, path: &str, key: Option<&str>) -> anyhow::Result<bool>;
}

pub struct SqliteStorage {
    pool: SqlitePool,
    threshold: usize,
    window: Duration,
}

impl SqliteStorage {
    pub async fn new(path: &str) -> anyhow::Result<Self> {
        if let Some(parent) = Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&format!("sqlite://{}", path))
            .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY,
                detail TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS requests (
                id INTEGER PRIMARY KEY,
                ip TEXT NOT NULL,
                path TEXT NOT NULL,
                key TEXT,
                ts INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await?;

        Ok(Self {
            pool,
            threshold: 30,
            window: Duration::from_secs(60),
        })
    }

    fn now_unix_ts() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
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

    async fn record_request(&self, ip: &str, path: &str, key: Option<&str>) -> anyhow::Result<()> {
        let ts = Self::now_unix_ts();

        sqlx::query("INSERT INTO requests (ip, path, key, ts) VALUES (?, ?, ?, ?)")
            .bind(ip)
            .bind(path)
            .bind(key)
            .bind(ts)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn is_abusive(&self, ip: &str, path: &str, key: Option<&str>) -> anyhow::Result<bool> {
        let cutoff = Self::now_unix_ts() - self.window.as_secs() as i64;

        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM requests WHERE ip = ? AND path = ? AND key IS ? AND ts >= ?",
        )
        .bind(ip)
        .bind(path)
        .bind(key)
        .bind(cutoff)
        .fetch_one(&self.pool)
        .await?;

        println!("IP {}; Hit count in this window: {}", ip, count.0);

        Ok(count.0 > self.threshold as i64)
    }
}
