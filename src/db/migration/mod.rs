//! Database migrations

use sea_orm_migration::prelude::*;

mod m20260227_022357_create_sources_storage;
mod m20260305_015253_create_news_storage;
mod m20260327_004812_create_signals_storage;

/// Database's `Migrator`.
///
/// For comprehensive database migrations management.
/// More capable than using [super::StorageConnection::has_pending_migrations()] and [super::StorageConnection::apply_migrations()].
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Migrator;

impl MigratorTrait for Migrator {
	fn migrations() -> Vec<Box<dyn MigrationTrait>> {
		vec![
			Box::new(m20260227_022357_create_sources_storage::Migration),
			Box::new(m20260305_015253_create_news_storage::Migration),
			Box::new(m20260327_004812_create_signals_storage::Migration),
		]
	}
}
