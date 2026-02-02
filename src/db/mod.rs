mod error;
mod impl_migration;
mod migration;

use sea_orm::{Database, DatabaseConnection};

pub use error::StorageError;
pub use migration::Migrator;

/// Database connection
#[derive(Clone, Debug)]
pub struct Connection {
	connection: DatabaseConnection,
}

impl Connection {
	/// Construct and create a new database connection pool.
	///
	/// Parent directories must exist, they will not be created when needed.
	/// If `database_file` is [`None`], a temporary in-memory database will be created.
	pub async fn new(database_file: Option<&std::path::Path>) -> Result<Self, StorageError> {
		let database_url = match database_file {
			Some(database_file) => {
				format!(
					"sqlite://{}?mode=rwc",
					std::path::absolute(database_file)?
						.to_str()
						.ok_or(StorageError::InvalidUtf8Path)?
				)
			}
			None => "sqlite::memory:".to_owned(),
		};

		tracing::info!(?database_url, "creating database connection");

		let connection = Database::connect(database_url).await?;

		Ok(Self { connection })
	}
}
