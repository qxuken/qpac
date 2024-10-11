use std::collections::HashMap;

use tokio::sync::Mutex;

use crate::{error::AppError, pac::Pac};

use super::Storage;

#[derive(Debug, Default)]
pub struct MemoryStorage {
    hosts: Mutex<Vec<String>>,
    files: Mutex<HashMap<String, String>>,
    latest: Mutex<Option<String>>,
}

impl Storage for MemoryStorage {
    async fn all_hosts(&self) -> Result<Vec<String>, AppError> {
        Ok(self.hosts.lock().await.clone())
    }

    async fn get_file(&self, hash: impl Into<String>) -> Result<String, AppError> {
        self.files
            .lock()
            .await
            .get(&hash.into())
            .cloned()
            .ok_or(AppError::NotFound)
    }

    async fn get_file_latest(&self) -> Result<Pac, AppError> {
        let hash = self
            .latest
            .lock()
            .await
            .as_ref()
            .cloned()
            .ok_or(AppError::NotFound)?;
        let file = self
            .files
            .lock()
            .await
            .get(&hash)
            .cloned()
            .ok_or(AppError::NotFound)?;
        Ok(Pac::new(file, hash))
    }

    async fn upload_file(&self, pac: &Pac) -> Result<(), AppError> {
        self.files
            .lock()
            .await
            .insert(pac.hash.clone(), pac.file.clone());
        Ok(())
    }

    async fn set_latest(&self, hash: impl Into<String>) -> Result<(), AppError> {
        let mut l = self.latest.lock().await;
        *l = Some(hash.into());
        Ok(())
    }

    async fn add_host(&self, host: impl Into<String>) -> Result<(), AppError> {
        let host = host.into();
        let mut hosts = self.hosts.lock().await;
        if hosts.binary_search(&host).is_ok() {
            Err(AppError::PreconditionFailed(
                "Host already exists".to_string(),
            ))?
        };
        let idx = hosts.partition_point(|x| x <= &host);
        hosts.insert(idx, host);
        Ok(())
    }

    async fn remove_host(&self, host: impl Into<String>) -> Result<(), AppError> {
        let mut hosts = self.hosts.lock().await;
        let Ok(i) = hosts.binary_search(&host.into()) else {
            Err(AppError::NotFound)?
        };
        hosts.remove(i);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::error::Result;

    #[tokio::test]
    async fn adds_sorted() -> Result<()> {
        let storage = MemoryStorage::default();
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
        let storage = MemoryStorage::default();
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
        let storage = MemoryStorage::default();
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
        let storage = MemoryStorage::default();
        let test = vec!["a", "aa"];
        for s in test.into_iter() {
            storage.add_host(s).await?;
        }
        assert_eq!(storage.remove_host("ab").await, Err(AppError::NotFound));
        Ok(())
    }
}
