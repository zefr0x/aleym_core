//! Core library powering Aleym.

mod db;
mod error;

pub use db::Migrator as DbMigrator;
pub use error::Error;

pub struct Representative {
	storage: db::Connection,
}

impl Representative {
	pub async fn new(database_file: Option<&std::path::Path>) -> Result<Self, Error> {
		tracing::trace!("initializing new Aleym Representative");

		Ok(Self {
			storage: db::Connection::new(database_file).await?,
		})
	}

	/// Return `true` if we have any pending migrations.
	///
	/// Useful to prepare the user interface before starting to apply them.
	pub async fn has_pending_migrations(&self) -> Result<bool, Error> {
		Ok(self.storage.has_pending_migrations().await?)
	}

	/// Apply all pending migrations. If there is none, it will silently fail.
	///
	/// This should be executed once after every update to avoid errors.
	pub async fn apply_migrations(&self) -> Result<(), Error> {
		Ok(self.storage.apply_migrations().await?)
	}
}
