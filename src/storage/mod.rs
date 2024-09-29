use crate::error::AppError;

pub mod memory_storage;

pub trait Storage {
    fn all_hosts(&self) -> Result<Vec<String>, AppError>;
    fn add_host(&mut self, host: impl Into<String>) -> Result<(), AppError>;
    fn remove_host(&mut self, host: impl Into<String>) -> Result<(), AppError>;
}
