use std::{str::FromStr, time::Duration};

use sqlx::{
    migrate,
    sqlite::{
        SqliteAutoVacuum, SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions,
        SqliteSynchronous,
    },
    ConnectOptions, SqlitePool,
};
use tracing::log::LevelFilter;

use crate::{
    error::{AppError, Result},
    pac::Pac,
};

use super::Storage;

#[derive(Debug)]
pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    pub async fn new(url: &str) -> Result<Self> {
        let conf = SqliteConnectOptions::from_str(url)?
            .log_statements(LevelFilter::Trace)
            .journal_mode(SqliteJournalMode::Wal)
            .create_if_missing(true)
            .foreign_keys(true)
            .optimize_on_close(true, None)
            .synchronous(SqliteSynchronous::Normal)
            .auto_vacuum(SqliteAutoVacuum::Full)
            .busy_timeout(Duration::from_secs(3))
            .pragma("cache_size", "10000")
            .pragma("temp_store", "MEMORY")
            .pragma("encoding", "'UTF-8'")
            .pragma("mmap_size", "268435456");

        let pool = SqlitePoolOptions::new().connect_with(conf).await?;

        migrate!().run(&pool).await?;

        Ok(Self { pool })
    }
}

impl Storage for SqliteStorage {
    async fn all_hosts(&self) -> Result<Vec<String>, AppError> {
        let mut conn = self.pool.acquire().await?;
        let res = sqlx::query!("SELECT host FROM white_list;")
            .fetch_all(conn.as_mut())
            .await?
            .into_iter()
            .map(|r| r.host)
            .collect();
        Ok(res)
    }

    async fn get_file(&self, hash: impl Into<String>) -> Result<String, AppError> {
        let mut conn = self.pool.acquire().await?;
        let host = hash.into();
        let res = sqlx::query!("SELECT file FROM pac WHERE hash = ?;", host)
            .fetch_one(conn.as_mut())
            .await?;
        Ok(res.file)
    }

    async fn get_file_latest(&self) -> Result<Pac, AppError> {
        let mut conn = self.pool.acquire().await?;
        let conf = sqlx::query!("SELECT value FROM conf WHERE key = 'latest_pac_file';")
            .fetch_one(conn.as_mut())
            .await?;
        let res = sqlx::query!("SELECT file FROM pac WHERE hash = ?;", conf.value)
            .fetch_one(conn.as_mut())
            .await?;
        Ok(Pac::new(res.file, conf.value))
    }

    async fn upload_file(&self, pac: &Pac) -> Result<(), AppError> {
        let mut conn = self.pool.acquire().await?;
        sqlx::query!(
            r#"
INSERT INTO pac(hash, file) VALUES(?, ?)
    ON CONFLICT(hash) DO UPDATE SET file=excluded.file;"#,
            pac.hash,
            pac.file
        )
        .execute(conn.as_mut())
        .await?;
        Ok(())
    }

    async fn set_latest(&self, hash: impl Into<String>) -> Result<(), AppError> {
        let mut conn = self.pool.acquire().await?;
        let hash = hash.into();
        sqlx::query!(
            r#"
INSERT INTO conf(key, value) VALUES ('latest_pac_file', ?)
    ON CONFLICT(key) DO UPDATE SET value=excluded.value"#,
            hash
        )
        .execute(conn.as_mut())
        .await?;
        Ok(())
    }

    async fn add_host(&self, host: impl Into<String>) -> Result<(), AppError> {
        let mut conn = self.pool.acquire().await?;
        let host = host.into();
        sqlx::query!(
            "INSERT INTO white_list(host) VALUES (?) ON CONFLICT(host) DO NOTHING",
            host
        )
        .execute(conn.as_mut())
        .await?;
        Ok(())
    }

    async fn remove_host(&self, host: impl Into<String>) -> Result<(), AppError> {
        let mut conn = self.pool.acquire().await?;
        let host = host.into();
        sqlx::query!("DELETE FROM white_list WHERE host = ?", host)
            .execute(conn.as_mut())
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::error::Result;

    #[tokio::test]
    async fn adds_sorted() -> Result<()> {
        let storage = SqliteStorage::new("sqlite::memory:").await?;
        let test = vec![
            "a".to_string(),
            "aa".to_string(),
            "ab".to_string(),
            "abc".to_string(),
            "b".to_string(),
            "bac".to_string(),
            "sa".to_string(),
            "z".to_string(),
        ];
        for s in test.iter() {
            storage.add_host(s).await?;
        }
        let res = storage.all_hosts().await?;
        assert_eq!(res, test);
        Ok(())
    }

    #[tokio::test]
    async fn fails_to_add_non_uniq() -> Result<()> {
        let storage = SqliteStorage::new("sqlite::memory:").await?;
        let test = vec!["a", "aa"];
        for s in test.into_iter() {
            storage.add_host(s).await?;
        }
        assert_eq!(
            storage.add_host("aa").await,
            Err(AppError::PreconditionFailed(
                "Host already exists".to_string()
            ))
        );
        Ok(())
    }

    #[tokio::test]
    async fn remove_existing() -> Result<()> {
        let storage = SqliteStorage::new("sqlite::memory:").await?;
        let test = ["a", "aa", "ab"];
        for s in test.into_iter() {
            storage.add_host(s).await?;
        }
        storage.remove_host("ab").await?;
        storage.remove_host("aa").await?;
        let res = storage.all_hosts().await?;
        assert_eq!(res, vec!["a".to_string()]);
        Ok(())
    }

    #[tokio::test]
    async fn fails_to_remove_missing() -> Result<()> {
        let storage = SqliteStorage::new("sqlite::memory:").await?;
        let test = vec!["a", "aa"];
        for s in test.into_iter() {
            storage.add_host(s).await?;
        }
        assert_eq!(storage.remove_host("ab").await, Err(AppError::NotFound));
        Ok(())
    }
}
