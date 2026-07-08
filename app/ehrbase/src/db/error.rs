/// Errors produced by the persistence foundation.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    /// Database settings could not be loaded from the environment.
    #[error("database configuration: {0}")]
    Config(#[from] Box<figment::Error>),

    /// A driver/pool/query error from `sqlx`.
    #[error("database: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// A schema migration failed to apply.
    #[error("migration: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
}
