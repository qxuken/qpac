use crate::error::AppError;

use super::Storage;

#[derive(Debug, Default)]
pub struct MemoryStorage {
    hosts: Vec<String>,
}

impl Storage for MemoryStorage {
    fn all_hosts(&self) -> Result<Vec<String>, AppError> {
        Ok(self.hosts.clone())
    }

    fn add_host(&mut self, host: impl Into<String>) -> Result<(), AppError> {
        let host = host.into();
        if self.hosts.binary_search(&host).is_ok() {
            Err(AppError::PreconditionFailed(
                "Host already exists".to_string(),
            ))?
        };
        let idx = self.hosts.partition_point(|x| x <= &host);
        self.hosts.insert(idx, host);
        Ok(())
    }

    fn remove_host(&mut self, host: impl Into<String>) -> Result<(), AppError> {
        let Ok(i) = self.hosts.binary_search(&host.into()) else {
            Err(AppError::NotFound)?
        };
        self.hosts.remove(i);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::error::Result;

    #[test]
    fn adds_sorted() -> Result<()> {
        let mut storage = MemoryStorage::default();
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
            storage.add_host(s)?;
        }
        let res = storage.all_hosts()?;
        assert_eq!(res, test);
        Ok(())
    }

    #[test]
    fn fails_to_add_non_uniq() -> Result<()> {
        let mut storage = MemoryStorage::default();
        let test = vec!["a", "aa"];
        for s in test.into_iter() {
            storage.add_host(s)?;
        }
        assert_eq!(
            storage.add_host("aa"),
            Err(AppError::PreconditionFailed(
                "Host already exists".to_string()
            ))
        );
        Ok(())
    }

    #[test]
    fn remove_existing() -> Result<()> {
        let mut storage = MemoryStorage::default();
        let test = ["a", "aa", "ab"];
        for s in test.into_iter() {
            storage.add_host(s)?;
        }
        storage.remove_host("ab")?;
        storage.remove_host("aa")?;
        let res = storage.all_hosts()?;
        assert_eq!(res, vec!["a".to_string()]);
        Ok(())
    }

    #[test]
    fn fails_to_remove_missing() -> Result<()> {
        let mut storage = MemoryStorage::default();
        let test = vec!["a", "aa"];
        for s in test.into_iter() {
            storage.add_host(s)?;
        }
        assert_eq!(storage.remove_host("ab"), Err(AppError::NotFound));
        Ok(())
    }
}
