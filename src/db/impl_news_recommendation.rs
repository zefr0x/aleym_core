use sea_orm::{Condition, QueryOrder, QuerySelect, prelude::*, sea_query::Func};

use super::{
	StorageConnection, StorageError,
	entities::{news, prelude::*},
};

impl StorageConnection {
	/// Yields different results on each execution based on feedback signals and some randomness. While results may overlap
	/// when the `IsRead` status of news hasn't changed, feedback signals can reduce this possibility, delaying it from
	/// happening between consecutive executions to improve user experience.
	///
	/// There is no cursor, so the list should be fixed in size without paging or infinite scrolling.
	///
	/// Shouldn't be used frequently to update the list when the user may be focusing on it.
	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn get_news_recommendations(&self, limit: u64) -> Result<Vec<news::Model>, StorageError> {
		// TODO: Implement ML-based recommendations, currently it's just random.
		Ok(News::find()
			.filter(
				Condition::all()
					.add(news::Column::IsLatestVersion.eq(true))
					.add(news::Column::IsRead.eq(false)),
			)
			.order_by(Func::random(), sea_orm::Order::Asc)
			.limit(limit)
			.all(&self.connection)
			.await?)
	}
}
