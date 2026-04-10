use sea_orm_migration::{
	prelude::*,
	schema::{boolean, integer, json, text, tiny_integer, uuid},
};

#[derive(DeriveIden)]
pub enum SourceDirectory {
	Table,
	Id,
	ParentDirectory,
	Name,
	Description,
}

#[derive(DeriveIden)]
pub enum SourceCategory {
	Table,
	Id,
	Name,
	Description,
}

#[derive(DeriveIden)]
pub enum Source {
	Table,
	Id,
	ParentDirectory,
	Informant,
	InformantParameters,
	Network,
	NetworkParameters,
	Name,
	Description,
	IconUri,
	LogoUri,
	CustomId,
	IsEnabled,
	ProvidedTtl,
}

#[derive(DeriveIden)]
pub enum SourceToCategoryLink {
	Table,
	SourceId,
	CategoryId,
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		// Define `source_directory` table for directory-based management of sources
		manager
			.create_table(
				Table::create()
					.table(SourceDirectory::Table)
					.if_not_exists()
					.col(uuid(SourceDirectory::Id).primary_key())
					.col(uuid(SourceDirectory::ParentDirectory).null())
					.col(text(SourceDirectory::Name))
					.col(text(SourceDirectory::Description).null())
					.foreign_key(
						ForeignKey::create()
							.from(SourceDirectory::Table, SourceDirectory::ParentDirectory)
							.to(SourceDirectory::Table, SourceDirectory::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					// Ensure directory uniqueness
					.index(
						Index::create()
							.col(SourceDirectory::ParentDirectory)
							.col(SourceDirectory::Name)
							.unique(),
					)
					.to_owned(),
			)
			.await?;
		// Ensure uniqueness for root directories
		manager
			.create_index(
				Index::create()
					.if_not_exists()
					.name("unique_root_directories")
					.table(SourceDirectory::Table)
					.col(SourceDirectory::Name)
					.unique()
					.and_where(Expr::col(SourceDirectory::ParentDirectory).is_null())
					.to_owned(),
			)
			.await?;

		// Define `source_category` table for category-based management of sources
		manager
			.create_table(
				Table::create()
					.table(SourceCategory::Table)
					.if_not_exists()
					.col(uuid(SourceCategory::Id).primary_key())
					.col(text(SourceCategory::Name).unique_key())
					.col(text(SourceCategory::Description).null())
					.to_owned(),
			)
			.await?;

		// Define `source` table for sources storage
		manager
			.create_table(
				Table::create()
					.table(Source::Table)
					.if_not_exists()
					.col(uuid(Source::Id).primary_key())
					.col(uuid(Source::ParentDirectory))
					.col(tiny_integer(Source::Informant))
					.col(json(Source::InformantParameters))
					.col(tiny_integer(Source::Network))
					.col(json(Source::NetworkParameters).null())
					.col(text(Source::Name))
					.col(text(Source::Description).null())
					.col(text(Source::IconUri).null())
					.col(text(Source::LogoUri).null())
					.col(text(Source::CustomId).null().unique_key())
					.col(boolean(Source::IsEnabled))
					// NOTE: TTL duration is stored in seconds.
					.col(integer(Source::ProvidedTtl).null())
					.foreign_key(
						ForeignKey::create()
							.from(Source::Table, Source::ParentDirectory)
							.to(SourceDirectory::Table, SourceDirectory::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					// Ensure source uniqueness under directory
					.index(Index::create().col(Source::ParentDirectory).col(Source::Name).unique())
					.to_owned(),
			)
			.await?;

		// Link `source` with `source_category` table for M2M relationship
		manager
			.create_table(
				Table::create()
					.table(SourceToCategoryLink::Table)
					.if_not_exists()
					.col(uuid(SourceToCategoryLink::SourceId))
					.col(uuid(SourceToCategoryLink::CategoryId))
					.primary_key(
						Index::create()
							.col(SourceToCategoryLink::SourceId)
							.col(SourceToCategoryLink::CategoryId),
					)
					.foreign_key(
						ForeignKey::create()
							.from(SourceToCategoryLink::Table, SourceToCategoryLink::SourceId)
							.to(Source::Table, Source::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.foreign_key(
						ForeignKey::create()
							.from(SourceToCategoryLink::Table, SourceToCategoryLink::CategoryId)
							.to(SourceCategory::Table, SourceCategory::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_table(Table::drop().table(SourceToCategoryLink::Table).to_owned())
			.await?;

		manager
			.drop_table(Table::drop().table(Source::Table).to_owned())
			.await?;

		manager
			.drop_table(Table::drop().table(SourceCategory::Table).to_owned())
			.await?;

		manager
			.drop_table(Table::drop().table(SourceDirectory::Table).to_owned())
			.await?;

		Ok(())
	}
}
