pub use sea_orm::Order as SortOrder;
use sea_orm::{ActiveValue::Set, Condition, QueryOrder, QuerySelect, prelude::*};
use time::OffsetDateTime;

use super::{
	StorageConnection, StorageError,
	entities::{news, prelude::*, source, source_category},
};

pub const TIME_MIN: OffsetDateTime = time::OffsetDateTime::UNIX_EPOCH;
pub const TIME_MAX: OffsetDateTime =
	time::OffsetDateTime::new_in_offset(time::Date::MAX, time::Time::MAX, time::UtcOffset::UTC);

#[derive(Debug, Clone)]
pub struct DirectoryBasedNewsFilter {
	pub parent_directory: Uuid,
	pub recursive: bool,
}

#[derive(Debug, Clone)]
pub struct DirectoryOrCategoriesBasedNewsFilter {
	pub directory: Option<DirectoryBasedNewsFilter>,
	pub categories: Vec<Uuid>,
}

#[derive(Debug, Clone)]
pub enum NewsFilter {
	Source(Uuid),
	DirectoryOrCategories(DirectoryOrCategoriesBasedNewsFilter),
}

impl StorageConnection {
	/// Cursor is based on `FirstFetchedAt` value
	///
	/// Use [`TIME_MAX`] and [`TIME_MIN`] with a limit to get the first page, than use the result to progress further.
	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn get_news_with_source_filter(
		&self,
		filter: NewsFilter,
		sort_order: SortOrder,
		cursor_after: OffsetDateTime,
		cursor_before: OffsetDateTime,
		limit: u64,
	) -> Result<Vec<news::Model>, StorageError> {
		let result = match filter {
			NewsFilter::Source(source) => {
				News::find()
					.filter(
						news::Column::Source
							.eq(source)
							.and(news::Column::IsLatestVersion.eq(true)),
					)
					.order_by(news::Column::FirstFetchedAt, sort_order.clone())
					.order_by(news::Column::PublishedAt, sort_order.clone())
					.order_by(news::Column::UpdatedAt, sort_order.clone())
					.order_by(news::Column::Id, sort_order)
					.limit(limit)
					.cursor_by(news::Column::FirstFetchedAt)
					.after(cursor_after)
					.before(cursor_before)
					.all(&self.connection)
					.await?
			}
			NewsFilter::DirectoryOrCategories(filter) => {
				let mut condition = Condition::all().add(news::Column::IsLatestVersion.eq(true));

				if !filter.categories.is_empty() {
					condition = condition.add(source_category::Column::Id.is_in(filter.categories));
				}

				if let Some(directory_filter) = filter.directory {
					if directory_filter.recursive {
						tracing::trace!(filter.parent_directory=?directory_filter.parent_directory, "traversing descendant directories");

						// PERF: Consider optimizing this to a single query.
						let directories = self
							.get_directories_by_parent(directory_filter.parent_directory, true)
							.await?;

						condition = condition.add(
							source::Column::ParentDirectory.is_in(directories.iter().map(|directory| directory.id)),
						);
					} else {
						condition =
							condition.add(source::Column::ParentDirectory.eq(directory_filter.parent_directory));
					}
				}

				News::find()
					.find_also_related(Source)
					.and_also_related(SourceCategory)
					.filter(condition)
					.order_by(news::Column::FirstFetchedAt, sort_order.clone())
					.order_by(news::Column::PublishedAt, sort_order.clone())
					.order_by(news::Column::UpdatedAt, sort_order.clone())
					.order_by(news::Column::Id, sort_order)
					.limit(limit)
					.cursor_by(news::Column::FirstFetchedAt)
					.after(cursor_after)
					.before(cursor_before)
					.all(&self.connection)
					.await?
					.into_iter()
					.map(|n| n.0)
					.collect::<Vec<news::Model>>()
			}
		};

		Ok(result)
	}

	/// If some or all news in the provided list are already read or not read, it will silently return without errors.
	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn set_news_read(&self, news: Vec<Uuid>, is_read: bool) -> Result<(), StorageError> {
		News::update_many()
			.set(news::ActiveModel {
				is_read: Set(is_read),
				..Default::default()
			})
			.filter(news::Column::Id.is_in(news))
			.exec(&self.connection)
			.await?;

		Ok(())
	}
}
