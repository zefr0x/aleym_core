use sea_orm_migration::{
	prelude::*,
	schema::{boolean, text, timestamp_with_time_zone, uuid},
};

#[derive(DeriveIden)]
pub enum News {
	Table,
	Id,
	Source,
	SourceProvidedId,
	IsLatestVersion,
	// NOTE: This is infered by fetch order, not by publication time.
	PreviousVersion,
	Uri,
	Title,
	Summary,
	Content,
	PublishedAt,
	// NOTE: Source provided value, stored to track updates that happened with no fetch or history in our database.
	UpdatedAt,
	FirstFetchedAt,
	// NOTE: This is to track when news got disappeared from the source or went beyond the length of the provided window.
	LastFetchedAt,
	IsRead,
}

#[derive(DeriveIden)]
pub enum NewsLabel {
	Table,
	Id,
	Name,
	Description,
}

#[derive(DeriveIden)]
pub enum NewsToLabelLink {
	Table,
	NewsId,
	LabelId,
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		// Define `news` table for news storage
		manager
			.create_table(
				Table::create()
					.table(News::Table)
					.if_not_exists()
					.col(uuid(News::Id).primary_key())
					.col(uuid(News::Source))
					.col(text(News::SourceProvidedId).null())
					.col(boolean(News::IsLatestVersion))
					.col(uuid(News::PreviousVersion).null())
					.col(text(News::Uri).null())
					.col(text(News::Title))
					.col(text(News::Summary).null())
					.col(text(News::Content).null())
					.col(timestamp_with_time_zone(News::PublishedAt).null())
					.col(timestamp_with_time_zone(News::UpdatedAt).null())
					.col(timestamp_with_time_zone(News::FirstFetchedAt))
					.col(timestamp_with_time_zone(News::LastFetchedAt))
					.col(boolean(News::IsRead))
					// Ensure linear edit history
					.index(
						Index::create()
							.col(News::Source)
							.col(News::SourceProvidedId)
							.col(News::PreviousVersion)
							.unique(),
					)
					.foreign_key(
						ForeignKey::create()
							.from(News::Table, News::Source)
							.to("source", "id")
							.on_delete(ForeignKeyAction::Cascade),
					)
					.foreign_key(
						ForeignKey::create()
							.from(News::Table, News::PreviousVersion)
							.to(News::Table, News::Id)
							.on_delete(ForeignKeyAction::SetNull),
					)
					.to_owned(),
			)
			.await?;

		// Define `news_label` table for label-based management of news
		manager
			.create_table(
				Table::create()
					.table(NewsLabel::Table)
					.if_not_exists()
					.col(uuid(NewsLabel::Id).primary_key())
					.col(text(NewsLabel::Name).unique_key())
					.col(text(NewsLabel::Description).null())
					.to_owned(),
			)
			.await?;

		// Link `news` with `news_label` table for M2M relationship
		manager
			.create_table(
				Table::create()
					.table(NewsToLabelLink::Table)
					.if_not_exists()
					.col(uuid(NewsToLabelLink::NewsId))
					.col(uuid(NewsToLabelLink::LabelId))
					.primary_key(
						Index::create()
							.col(NewsToLabelLink::NewsId)
							.col(NewsToLabelLink::LabelId),
					)
					.foreign_key(
						ForeignKey::create()
							.from(NewsToLabelLink::Table, NewsToLabelLink::NewsId)
							.to(News::Table, News::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.foreign_key(
						ForeignKey::create()
							.from(NewsToLabelLink::Table, NewsToLabelLink::LabelId)
							.to(NewsLabel::Table, NewsLabel::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager.drop_table(Table::drop().table(News::Table).to_owned()).await?;
		for table in [
			News::Table.into_table_ref(),
			NewsLabel::Table.into_table_ref(),
			NewsToLabelLink::Table.into_table_ref(),
		] {
			manager.drop_table(Table::drop().table(table).to_owned()).await?;
		}

		Ok(())
	}
}
