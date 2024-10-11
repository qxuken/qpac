use crate::{error::AppError, pac::Pac};

pub mod memory_storage;

pub trait Storage {
    fn all_hosts(&self) -> impl futures::Future<Output = Result<Vec<String>, AppError>>;

    fn get_file(
        &self,
        hash: impl Into<String>,
    ) -> impl futures::Future<Output = Result<String, AppError>>;
    fn get_file_latest(&self) -> impl futures::Future<Output = Result<Pac, AppError>>;
    fn upload_file(&self, file: &Pac) -> impl futures::Future<Output = Result<(), AppError>>;
    fn set_latest(
        &self,
        hash: impl Into<String>,
    ) -> impl futures::Future<Output = Result<(), AppError>>;

    fn add_host(
        &self,
        host: impl Into<String>,
    ) -> impl futures::Future<Output = Result<(), AppError>>;
    fn remove_host(
        &self,
        host: impl Into<String>,
    ) -> impl futures::Future<Output = Result<(), AppError>>;
}
