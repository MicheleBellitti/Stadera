//! Stadera storage layer: Postgres repositories for the domain types.
//!
//! Boundary between pure domain (`stadera-domain`) and the database.
//! All I/O lives here.

pub mod error;
pub mod repositories;
mod rows;

pub use error::{StorageError, StorageResult};
pub use repositories::session::Session;
pub use repositories::user::User;
pub use repositories::withings_credentials::WithingsCredentials;

use sqlx::PgPool;

use repositories::{
    measurement::PgMeasurementRepository, session::PgSessionRepository, user::PgUserRepository,
    user_profile::PgUserProfileRepository, withings_credentials::PgWithingsCredentialsRepository,
};

pub struct StorageContext {
    pool: PgPool,
}

impl StorageContext {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn measurements(&self) -> PgMeasurementRepository<'_> {
        PgMeasurementRepository::new(&self.pool)
    }
    pub fn users(&self) -> PgUserRepository<'_> {
        PgUserRepository::new(&self.pool)
    }
    pub fn user_profiles(&self) -> PgUserProfileRepository<'_> {
        PgUserProfileRepository::new(&self.pool)
    }
    pub fn withings_credentials(&self) -> PgWithingsCredentialsRepository<'_> {
        PgWithingsCredentialsRepository::new(&self.pool)
    }

    pub fn sessions(&self) -> PgSessionRepository<'_> {
        PgSessionRepository::new(&self.pool)
    }
}
