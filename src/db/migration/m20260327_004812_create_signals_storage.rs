use sea_orm_migration::{
	prelude::*,
	schema::{boolean, integer, timestamp_with_time_zone, tiny_integer, uuid},
};

// NOTE: Duration values are stored in milliseconds (ms).

#[derive(DeriveIden)]
pub enum SourceFetchSignal {
	Table,
	Id,
	Source,
	DoneAt,
	Duration,
	FailureCode,
	NewItemsCount,
	// NOTE: Those apply only to newly added items.
	LatestPublishAt,
	OldestPublishAt,
}

#[derive(DeriveIden)]
pub enum NewsApearanceSignal {
	Table,
	Id,
	News,
	HappenedAt,
	Duration,
}

#[derive(DeriveIden)]
pub enum NewsFocusSignal {
	Table,
	Id,
	News,
	DoneAt,
	Duration,
}

#[derive(DeriveIden)]
pub enum NewsReadSignal {
	Table,
	Id,
	News,
	DoneAt,
	Duration,
	ScrollDepthPercentage,
}

#[derive(DeriveIden)]
pub enum NewsExplicitVoteSignal {
	Table,
	Id,
	News,
	DoneAt,
	IsUpVote,
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(SourceFetchSignal::Table)
					.if_not_exists()
					.col(uuid(SourceFetchSignal::Id).primary_key())
					.col(uuid(SourceFetchSignal::Source))
					.col(timestamp_with_time_zone(SourceFetchSignal::DoneAt))
					.col(integer(SourceFetchSignal::Duration))
					.col(tiny_integer(SourceFetchSignal::FailureCode).null())
					.col(integer(SourceFetchSignal::NewItemsCount))
					.col(timestamp_with_time_zone(SourceFetchSignal::LatestPublishAt).null())
					.col(timestamp_with_time_zone(SourceFetchSignal::OldestPublishAt).null())
					.foreign_key(
						ForeignKey::create()
							.from(SourceFetchSignal::Table, SourceFetchSignal::Source)
							.to("source", "id")
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await?;

		manager
			.create_table(
				Table::create()
					.table(NewsApearanceSignal::Table)
					.if_not_exists()
					.col(uuid(NewsApearanceSignal::Id).primary_key())
					.col(uuid(NewsApearanceSignal::News))
					.col(timestamp_with_time_zone(NewsApearanceSignal::HappenedAt))
					.col(integer(NewsApearanceSignal::Duration))
					.foreign_key(
						ForeignKey::create()
							.from(NewsApearanceSignal::Table, NewsApearanceSignal::News)
							.to("news", "id")
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await?;

		manager
			.create_table(
				Table::create()
					.table(NewsFocusSignal::Table)
					.if_not_exists()
					.col(uuid(NewsFocusSignal::Id).primary_key())
					.col(uuid(NewsFocusSignal::News))
					.col(timestamp_with_time_zone(NewsFocusSignal::DoneAt))
					.col(integer(NewsFocusSignal::Duration))
					.foreign_key(
						ForeignKey::create()
							.from(NewsFocusSignal::Table, NewsFocusSignal::News)
							.to("news", "id")
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await?;

		manager
			.create_table(
				Table::create()
					.table(NewsReadSignal::Table)
					.if_not_exists()
					.col(uuid(NewsReadSignal::Id).primary_key())
					.col(uuid(NewsReadSignal::News))
					.col(timestamp_with_time_zone(NewsReadSignal::DoneAt))
					.col(integer(NewsReadSignal::Duration))
					.col(tiny_integer(NewsReadSignal::ScrollDepthPercentage))
					.foreign_key(
						ForeignKey::create()
							.from(NewsReadSignal::Table, NewsReadSignal::News)
							.to("news", "id")
							.on_delete(ForeignKeyAction::Cascade),
					)
					.check(Check::unnamed(
						Expr::col(NewsReadSignal::ScrollDepthPercentage).between(0, 100),
					))
					.to_owned(),
			)
			.await?;

		manager
			.create_table(
				Table::create()
					.table(NewsExplicitVoteSignal::Table)
					.if_not_exists()
					.col(uuid(NewsExplicitVoteSignal::Id).primary_key())
					.col(uuid(NewsExplicitVoteSignal::News))
					.col(timestamp_with_time_zone(NewsExplicitVoteSignal::DoneAt))
					.col(boolean(NewsExplicitVoteSignal::IsUpVote))
					.foreign_key(
						ForeignKey::create()
							.from(NewsExplicitVoteSignal::Table, NewsExplicitVoteSignal::News)
							.to("news", "id")
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		for table in [
			SourceFetchSignal::Table.into_table_ref(),
			NewsApearanceSignal::Table.into_table_ref(),
			NewsFocusSignal::Table.into_table_ref(),
			NewsReadSignal::Table.into_table_ref(),
			NewsExplicitVoteSignal::Table.into_table_ref(),
		] {
			manager.drop_table(Table::drop().table(table).to_owned()).await?;
		}

		Ok(())
	}
}
