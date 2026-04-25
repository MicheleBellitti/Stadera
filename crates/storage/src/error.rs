use thiserror::Error;

pub type StorageResult<T> = Result<T, StorageError>;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("data corruption in {table}: {msg}")]
    Corruption { table: &'static str, msg: String },
}

impl StorageError {
    pub(crate) fn corruption(table: &'static str, msg: impl Into<String>) -> Self {
        Self::Corruption {
            table,
            msg: msg.into(),
        }
    }
}
