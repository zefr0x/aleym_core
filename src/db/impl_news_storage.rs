use sea_orm::{ActiveValue::Set, Condition, QuerySelect, prelude::*, sea_query::LikeExpr};
use time::OffsetDateTime;

use super::{
	SortOrder, StorageConnection, StorageError,
	entities::{news, prelude::*, source, source_category},
};

pub const TIME_MIN: OffsetDateTime = time::OffsetDateTime::UNIX_EPOCH;
pub const TIME_MAX: OffsetDateTime =
	time::OffsetDateTime::new_in_offset(time::Date::MAX, time::Time::MAX, time::UtcOffset::UTC);

#[derive(Debug, Clone)]
pub struct BySourceDirectory {
	pub parent_directory: Uuid,
	pub recursive: bool,
}

#[derive(Debug, Clone)]
pub enum BySources {
	Identifiers(Vec<Uuid>),
	Scope {
		directory: Option<BySourceDirectory>,
		categories: Vec<Uuid>,
	},
}

#[derive(Debug, Clone, Default)]
pub struct NewsFilter {
	pub text: Option<String>,
	pub sources: Option<BySources>,
}

#[cfg(feature = "_informant")]
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub(crate) struct InputNews {
	pub(crate) source_provided_id: Option<String>,
	pub(crate) uri: Option<String>,
	pub(crate) title: String,
	pub(crate) summary: Option<String>,
	pub(crate) content: Option<String>,
	pub(crate) published_at: Option<OffsetDateTime>,
	pub(crate) updated_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddingNewsOutput {
	/// Newly added news items or new versions of existing ones.
	pub new: Vec<Uuid>,
	/// Were seen again in the feed without any change or there is a newer version of them identified in `new`.
	pub touched: Vec<Uuid>,
	/// Time of latest publication in news items.
	pub latest_publish: Option<OffsetDateTime>,
	/// Time of oldest publication in news items.
	pub oldest_publish: Option<OffsetDateTime>,
}

impl StorageConnection {
	#[cfg(feature = "_informant")]
	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub(crate) async fn add_news(&self, source: Uuid, items: Vec<InputNews>) -> Result<AddingNewsOutput, StorageError> {
		use sea_orm::TransactionTrait as _;

		let now = OffsetDateTime::now_utc();

		// Group items by to detect multiple versions in the same batch
		let mut grouped_items = std::collections::HashMap::new();

		for item in items {
			grouped_items
				.entry(item.source_provided_id.clone())
				// Avoiding the need to handle duplicate items
				.or_insert_with(std::collections::HashSet::new)
				.insert(item);
		}

		let mut models_to_insert = vec![];
		let mut previous_to_update_latest = vec![];
		let mut existing_to_update_last_fetch = vec![];

		let mut output = AddingNewsOutput {
			new: Vec::new(),
			touched: Vec::new(),
			latest_publish: None,
			oldest_publish: None,
		};

		macro_rules! update_output_publish_bounds {
			($item:expr, $output:expr) => {{
				match ($item.published_at, $output.oldest_publish, $output.latest_publish) {
					(Some(published_at), _, Some(latest_publish)) if published_at > latest_publish => {
						$output.latest_publish = $item.published_at;
					}
					(Some(published_at), Some(oldest_publish), _) if published_at < oldest_publish => {
						$output.oldest_publish = $item.published_at;
					}
					(Some(_), None, None) => {
						$output.oldest_publish = $item.published_at;
						$output.latest_publish = $item.published_at;
					}
					_ => {}
				}
			}};
		}

		for (key_source_provided_id, versions) in grouped_items {
			// Skip versioning items when there is no `source_provided_id`
			if key_source_provided_id.is_none() {
				for item in versions {
					// PERF: Optimize checking for existing news to be in a single query.
					// Check if it previously exist
					if let Some(exists) = News::find()
						.filter(
							Condition::all()
								.add(news::Column::Source.eq(source))
								.add(news::Column::SourceProvidedId.is_null())
								.add(news::Column::IsLatestVersion.eq(true))
								.add(news::Column::PreviousVersion.is_null())
								.add(news::Column::Uri.eq(item.uri.clone()))
								.add(news::Column::Title.eq(item.title.clone()))
								.add(news::Column::Summary.eq(item.summary.clone()))
								.add(news::Column::Content.eq(item.content.clone()))
								.add(news::Column::PublishedAt.eq(item.published_at))
								.add(news::Column::UpdatedAt.eq(item.updated_at)),
						)
						.one(&self.connection)
						.await?
					{
						existing_to_update_last_fetch.push(exists.id);
						output.touched.push(exists.id);

						tracing::debug!(news.id = ?exists.id, "touching existing news");
					} else {
						let id = Uuid::new_v4();

						models_to_insert.push(news::ActiveModel {
							id: Set(id),
							source: Set(source),
							source_provided_id: Set(item.source_provided_id),
							is_latest_version: Set(true),
							previous_version: Set(None),
							uri: Set(item.uri),
							title: Set(item.title),
							summary: Set(item.summary),
							content: Set(item.content),
							published_at: Set(item.published_at),
							updated_at: Set(item.updated_at),
							first_fetched_at: Set(now),
							last_fetched_at: Set(now),
							is_read: Set(false),
						});
						output.new.push(id);

						update_output_publish_bounds!(item, output);

						tracing::debug!(news.id = ?id, "adding new news");
					}
				}
			} else {
				let mut versions = versions.into_iter().collect::<Vec<InputNews>>();
				// Sort versions in ascending order to process oldest first
				versions.sort_by_key(|v| v.updated_at.or(v.published_at));

				// NOTE: If old items got added when newer ones exists, they will be considered as new versions.

				// PERF: Optimize checking for previous versions of news to be in a single query.
				let previous = News::find()
					.filter(
						Condition::all()
							.add(news::Column::Source.eq(source))
							.add(news::Column::SourceProvidedId.eq(key_source_provided_id))
							.add(news::Column::IsLatestVersion.eq(true)),
					)
					.one(&self.connection)
					.await?;

				// If news hasn't change update `last_fetched_at`
				if let Some(previous) = &previous
					&& previous.published_at == versions[0].published_at
					&& previous.updated_at == versions[0].updated_at
					&& previous.uri == versions[0].uri
					&& previous.title == versions[0].title
					&& previous.summary == versions[0].summary
					&& previous.content == versions[0].content
				{
					existing_to_update_last_fetch.push(previous.id);
					output.touched.push(previous.id);

					versions.remove(0);

					tracing::debug!(news.id = ?previous.id, "touching existing news");
				}

				let mut previous_id = previous.as_ref().map(|n| n.id);

				let items_count = versions.len();

				for (index, item) in versions.into_iter().enumerate() {
					let id = Uuid::new_v4();

					let is_latest = index == items_count - 1;

					models_to_insert.push(news::ActiveModel {
						id: Set(id),
						source: Set(source),
						source_provided_id: Set(item.source_provided_id),
						is_latest_version: Set(is_latest),
						previous_version: Set(previous_id),
						uri: Set(item.uri),
						title: Set(item.title),
						summary: Set(item.summary),
						content: Set(item.content),
						published_at: Set(item.published_at),
						updated_at: Set(item.updated_at),
						first_fetched_at: Set(now),
						last_fetched_at: Set(now),
						is_read: Set(false),
					});
					output.new.push(id);

					update_output_publish_bounds!(item, output);

					if let Some(previous_id) = previous_id.take()
						&& !output.new.contains(&previous_id)
					{
						previous_to_update_latest.push(previous_id);
						output.touched.push(previous_id);
					}

					if !is_latest {
						previous_id = Some(id);
					}

					tracing::debug!(news.id = ?id, is_latest, "adding new versioned news");
				}
			}
		}

		self.connection
			.transaction(|transaction| {
				Box::pin(async move {
					if !existing_to_update_last_fetch.is_empty() {
						News::update_many()
							.col_expr(news::Column::LastFetchedAt, Expr::value(now))
							.filter(news::Column::Id.is_in(existing_to_update_last_fetch))
							.exec(transaction)
							.await?;
					}

					News::insert_many(models_to_insert).exec(transaction).await?;

					if !previous_to_update_latest.is_empty() {
						News::update_many()
							.col_expr(news::Column::IsLatestVersion, Expr::value(false))
							.filter(news::Column::Id.is_in(previous_to_update_latest))
							.exec(transaction)
							.await?;
					}

					Ok(())
				})
			})
			.await?;

		Ok(output)
	}

	/// Cursor is based on `FirstFetchedAt` value
	///
	/// Use [`TIME_MAX`] and [`TIME_MIN`] with a limit to get the first page, than use the result to progress further.
	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn get_news_with_filter(
		&self,
		filter: NewsFilter,
		sort_order: SortOrder,
		cursor_after: OffsetDateTime,
		cursor_before: OffsetDateTime,
		limit: u64,
	) -> Result<Vec<news::Model>, StorageError> {
		let mut condition = Condition::all().add(news::Column::IsLatestVersion.eq(true));

		if let Some(mut text) = filter.text {
			text = text.replace("%", "\\%").replace("_", "\\_");
			let pattern = LikeExpr::new(format!(r"%{text}%")).escape('\\');

			condition = condition.add(
				Condition::any()
					.add(news::Column::Title.like(pattern.clone()))
					.add(news::Column::Summary.like(pattern.clone()))
					.add(news::Column::Content.like(pattern)),
			);
		}

		let result = match filter.sources {
			None => {
				let mut stmt = News::find().filter(condition).limit(limit).cursor_by((
					news::Column::FirstFetchedAt,
					news::Column::PublishedAt,
					news::Column::UpdatedAt,
					// Ensure that we have a consistent order when there is nothing for fallback sorting
					news::Column::Id,
				));

				match sort_order {
					SortOrder::Ascending => stmt
						.after((cursor_after, TIME_MIN, TIME_MIN, Uuid::nil()))
						.before((cursor_before, TIME_MAX, TIME_MAX, Uuid::max()))
						.asc(),
					SortOrder::Descending => stmt
						.after((cursor_before, TIME_MAX, TIME_MAX, Uuid::max()))
						.before((cursor_after, TIME_MIN, TIME_MIN, Uuid::nil()))
						.desc(),
				}
				.all(&self.connection)
				.await?
			}
			Some(BySources::Identifiers(sources)) => {
				condition = condition.add(news::Column::Source.is_in(sources));

				let mut stmt = News::find().filter(condition).limit(limit).cursor_by((
					news::Column::FirstFetchedAt,
					news::Column::PublishedAt,
					news::Column::UpdatedAt,
					// Ensure that we have a consistent order when there is nothing for fallback sorting
					news::Column::Id,
				));

				match sort_order {
					SortOrder::Ascending => stmt
						.after((cursor_after, TIME_MIN, TIME_MIN, Uuid::nil()))
						.before((cursor_before, TIME_MAX, TIME_MAX, Uuid::max()))
						.asc(),
					SortOrder::Descending => stmt
						.after((cursor_before, TIME_MAX, TIME_MAX, Uuid::max()))
						.before((cursor_after, TIME_MIN, TIME_MIN, Uuid::nil()))
						.desc(),
				}
				.all(&self.connection)
				.await?
			}
			Some(BySources::Scope {
				directory: directory_filter,
				categories: categories_filter,
			}) => {
				if !categories_filter.is_empty() {
					condition = condition.add(source_category::Column::Id.is_in(categories_filter));
				}

				if let Some(directory_filter) = directory_filter {
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

				let mut stmt = News::find()
					.find_also_related(Source)
					.and_also_related(SourceCategory)
					.filter(condition)
					.limit(limit)
					.cursor_by((
						news::Column::FirstFetchedAt,
						news::Column::PublishedAt,
						news::Column::UpdatedAt,
						// Ensure that we have a consistent order when there is nothing for fallback sorting
						news::Column::Id,
					));

				match sort_order {
					SortOrder::Ascending => stmt
						.after((cursor_after, TIME_MIN, TIME_MIN, Uuid::nil()))
						.before((cursor_before, TIME_MAX, TIME_MAX, Uuid::max()))
						.asc(),
					SortOrder::Descending => stmt
						.after((cursor_before, TIME_MAX, TIME_MAX, Uuid::max()))
						.before((cursor_after, TIME_MIN, TIME_MIN, Uuid::nil()))
						.desc(),
				}
				.all(&self.connection)
				.await?
				.into_iter()
				.map(|n| n.0)
				.collect::<Vec<news::Model>>()
			}
		};

		Ok(result)
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn get_news(&self, id: Uuid) -> Result<news::Model, StorageError> {
		Ok(News::find_by_id(id)
			.one(&self.connection)
			.await?
			.ok_or(DbErr::RecordNotFound(format!("Expected news with id = `{id}`")))?)
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
