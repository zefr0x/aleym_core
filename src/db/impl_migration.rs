use super::StorageConnection;
use super::error::StorageError;
use super::migration::Migrator;

use sea_orm_migration::MigratorTrait;

impl StorageConnection {
	/// Return `true` if we have any pending database migrations.
	///
	/// Useful to prepare the user interface before starting to apply them.
	pub async fn has_pending_migrations(&self) -> Result<bool, StorageError> {
		Ok(!Migrator::get_pending_migrations(&self.connection).await?.is_empty())
	}

	/// Apply all pending database migrations. If there is none, it will silently fail.
	///
	/// This should be executed once after every update to avoid errors.
	pub async fn apply_migrations(&self) -> Result<(), StorageError> {
		tracing::debug!(
			number_of_applied_migrations = Migrator::get_applied_migrations(&self.connection).await?.len(),
			number_of_pending_migrations = Migrator::get_pending_migrations(&self.connection).await?.len(),
			"migrating the database"
		);

		// FIX: Return feedback as error if noting to be done, it doesn't seem to be supported by sea-orm-migration.
		Ok(Migrator::up(&self.connection, None).await?)
	}
}

#[cfg(test)]
pub(crate) mod tests {
	use super::StorageConnection;
	use tracing_test::traced_test;

	pub(crate) async fn test_connection_and_migrations() -> StorageConnection {
		let con = StorageConnection::new(None).await.unwrap();

		if con
			.has_pending_migrations()
			.await
			.expect("Failed to check for pending migrations")
		{
			con.apply_migrations().await.expect("Failed to apply migrations");
		} else {
			panic!("Expected pending database migrations")
		}

		con
	}

	#[tokio::test]
	#[traced_test]
	async fn database_migrations() {
		test_connection_and_migrations().await;
	}
}
