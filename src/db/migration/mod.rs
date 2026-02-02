//! Database migrations

use sea_orm_migration::prelude::*;

/// Database's `Migrator`.
///
/// For comprehensive database migrations management.
/// More capable than using [crate::Representative::has_pending_migrations()] and [crate::Representative::apply_migrations()].
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Migrator;

impl MigratorTrait for Migrator {
	fn migrations() -> Vec<Box<dyn MigrationTrait>> {
		vec![]
	}
}
