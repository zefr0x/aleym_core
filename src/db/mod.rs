#[expect(unused)]
mod entities;
mod error;
mod impl_migration;
mod impl_news_recommendation;
mod impl_news_storage;
mod impl_signals_storage;
mod impl_source_storage;
mod migration;

pub use sea_orm::ActiveValue;
use sea_orm::{Database, DatabaseConnection};
pub use time;
pub use uuid;

pub use error::StorageError;
pub use impl_news_storage::*;
pub use impl_signals_storage::*;
pub use migration::Migrator;

#[derive(Clone, Debug)]
pub enum SortOrder {
	Ascending,
	Descending,
}

#[derive(Clone, Debug)]
pub struct StorageConnection {
	connection: DatabaseConnection,
}

impl StorageConnection {
	/// Construct and create a new database connection pool.
	///
	/// Parent directories must exist, they will not be created when needed.
	/// If `database_file` is [`None`], a temporary in-memory database will be created.
	pub(crate) async fn new(database_file: Option<&std::path::Path>) -> Result<Self, StorageError> {
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
