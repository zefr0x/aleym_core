use super::Connection;
use super::error::StorageError;
use super::migration::Migrator;

use sea_orm_migration::MigratorTrait;

impl Connection {
	/// Return `true` if we have any pending database migrations.
	pub async fn has_pending_migrations(&self) -> Result<bool, StorageError> {
		Ok(!Migrator::get_pending_migrations(&self.connection).await?.is_empty())
	}

	/// Apply all pending database migrations. If there is none, it will silently fail.
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
mod tests {
	use super::*;
	use tracing_test::traced_test;

	#[tokio::test]
	#[traced_test]
	async fn apply_database_migrations() {
		let con = Connection::new(None).await.unwrap();

		if con.has_pending_migrations().await.unwrap() {
			con.apply_migrations().await.unwrap();
		}
	}
}
