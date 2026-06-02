use sea_orm::{ActiveValue::Set, Paginator, QuerySelect, SelectModel, prelude::*};
#[cfg(feature = "_informant")]
use sea_orm::{Condition, IntoSimpleExpr};
use time::{Duration, OffsetDateTime};

#[cfg(feature = "_informant")]
use super::entities::source_fetch_signal;
use super::{
	StorageConnection, StorageError,
	entities::{
		news, news_apearance_signal, news_explicit_vote_signal, news_focus_signal, news_read_signal, prelude::*,
	},
};

#[cfg(feature = "_informant")]
#[derive(Debug)]
pub(crate) enum SourceFeedbackSignal {
	FetchSignal {
		source: Uuid,
		done_at: OffsetDateTime,
		duration: Duration,
		failure_code: Option<crate::inform::InformantErrorKind>,
		new_items_count: i32,
		latest_publish_at: Option<OffsetDateTime>,
		oldest_publish_at: Option<OffsetDateTime>,
	},
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum UserFeedbackSignal {
	/// News item got displayed to the user.
	NewsApearanceSignal {
		news: Uuid,
		happened_at: OffsetDateTime,
		duration: Duration,
	},
	/// User focused or hovered on news cover.
	NewsFocusSignal {
		news: Uuid,
		done_at: OffsetDateTime,
		duration: Duration,
	},
	/// User opened and read the news.
	///
	/// `scroll_depth_percentage` should indicate how much of the news got displayed to the user.
	NewsReadSignal {
		news: Uuid,
		done_at: OffsetDateTime,
		duration: Duration,
		scroll_depth_percentage: i8,
	},
	/// User explicitly voted for news.
	NewsExplicitVoteSignal {
		news: Uuid,
		done_at: OffsetDateTime,
		is_up_vote: bool,
	},
}

impl StorageConnection {
	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn store_user_feedback_signal(&self, signal: UserFeedbackSignal) -> Result<(), StorageError> {
		match signal {
			UserFeedbackSignal::NewsApearanceSignal {
				news,
				happened_at,
				duration,
			} => {
				news_apearance_signal::ActiveModel {
					id: Set(Uuid::new_v4()),
					news: Set(news),
					happened_at: Set(happened_at),
					duration: Set(duration.whole_milliseconds().try_into()?),
				}
				.insert(&self.connection)
				.await?;
			}

			UserFeedbackSignal::NewsFocusSignal {
				news,
				done_at,
				duration,
			} => {
				news_focus_signal::ActiveModel {
					id: Set(Uuid::new_v4()),
					news: Set(news),
					done_at: Set(done_at),
					duration: Set(duration.whole_milliseconds().try_into()?),
				}
				.insert(&self.connection)
				.await?;
			}

			UserFeedbackSignal::NewsReadSignal {
				news,
				done_at,
				duration,
				scroll_depth_percentage,
			} => {
				if !(0..=100).contains(&scroll_depth_percentage) {
					Err(StorageError::InvalidPercentageNumber(scroll_depth_percentage))?
				}

				news_read_signal::ActiveModel {
					id: Set(Uuid::new_v4()),
					news: Set(news),
					done_at: Set(done_at),
					duration: Set(duration.whole_milliseconds().try_into()?),
					scroll_depth_percentage: Set(scroll_depth_percentage),
				}
				.insert(&self.connection)
				.await?;
			}

			UserFeedbackSignal::NewsExplicitVoteSignal {
				news,
				done_at,
				is_up_vote,
			} => {
				news_explicit_vote_signal::ActiveModel {
					id: Set(Uuid::new_v4()),
					news: Set(news),
					done_at: Set(done_at),
					is_up_vote: Set(is_up_vote),
				}
				.insert(&self.connection)
				.await?;
			}
		}

		Ok(())
	}

	#[cfg(feature = "_informant")]
	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub(crate) async fn store_source_feedback_signal(&self, signal: SourceFeedbackSignal) -> Result<(), StorageError> {
		match signal {
			SourceFeedbackSignal::FetchSignal {
				source,
				done_at,
				duration,
				failure_code,
				new_items_count,
				latest_publish_at,
				oldest_publish_at,
			} => {
				source_fetch_signal::ActiveModel {
					id: Set(Uuid::new_v4()),
					source: Set(source),
					done_at: Set(done_at),
					duration: Set(duration.whole_milliseconds().try_into()?),
					failure_code: Set(failure_code.map(|k| k.into())),
					new_items_count: Set(new_items_count),
					latest_publish_at: Set(latest_publish_at),
					oldest_publish_at: Set(oldest_publish_at),
				}
				.insert(&self.connection)
				.await?;
			}
		}

		Ok(())
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub(crate) async fn get_focus_signals<'db>(
		&'db self,
		source: uuid::Uuid,
		cutoff_time: time::OffsetDateTime,
		limit: u64,
	) -> Result<Paginator<'db, DatabaseConnection, SelectModel<news_focus_signal::Model>>, StorageError> {
		Ok(NewsFocusSignal::find()
			.has_related(News, news::Column::Source.eq(source))
			.filter(news_focus_signal::Column::DoneAt.gte(cutoff_time))
			.limit(limit)
			.paginate(&self.connection, 100))
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub(crate) async fn get_read_signals<'db>(
		&'db self,
		source: uuid::Uuid,
		cutoff_time: time::OffsetDateTime,
		limit: u64,
	) -> Result<Paginator<'db, DatabaseConnection, SelectModel<news_read_signal::Model>>, StorageError> {
		Ok(NewsReadSignal::find()
			.has_related(News, news::Column::Source.eq(source))
			.filter(news_read_signal::Column::DoneAt.gte(cutoff_time))
			.limit(limit)
			.paginate(&self.connection, 100))
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub(crate) async fn get_vote_signals<'db>(
		&'db self,
		source: uuid::Uuid,
		cutoff_time: time::OffsetDateTime,
		limit: u64,
	) -> Result<Paginator<'db, DatabaseConnection, SelectModel<news_explicit_vote_signal::Model>>, StorageError> {
		Ok(NewsExplicitVoteSignal::find()
			.has_related(News, news::Column::Source.eq(source))
			.filter(news_explicit_vote_signal::Column::DoneAt.gte(cutoff_time))
			.limit(limit)
			.paginate(&self.connection, 100))
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub(crate) async fn get_source_appearance_signals<'db>(
		&'db self,
		source: uuid::Uuid,
		cutoff_time: time::OffsetDateTime,
		limit: u64,
	) -> Result<Paginator<'db, DatabaseConnection, SelectModel<news_apearance_signal::Model>>, StorageError> {
		Ok(NewsApearanceSignal::find()
			.has_related(News, news::Column::Source.eq(source))
			.filter(news_apearance_signal::Column::HappenedAt.gte(cutoff_time))
			.limit(limit)
			.paginate(&self.connection, 100))
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub(crate) async fn get_news_appearance_signals<'db>(
		&'db self,
		news: uuid::Uuid,
		cutoff_time: time::OffsetDateTime,
		limit: u64,
	) -> Result<Paginator<'db, DatabaseConnection, SelectModel<news_apearance_signal::Model>>, StorageError> {
		Ok(NewsApearanceSignal::find()
			.has_related(News, news::Column::Id.eq(news))
			.filter(news_apearance_signal::Column::HappenedAt.gte(cutoff_time))
			.limit(limit)
			.paginate(&self.connection, 100))
	}

	#[cfg(feature = "_informant")]
	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub(crate) async fn get_source_fetch_signals<'db>(
		&'db self,
		source: Option<Uuid>,
		cutoff_time: time::OffsetDateTime,
		limit: u64,
	) -> Result<Paginator<'db, DatabaseConnection, SelectModel<source_fetch_signal::Model>>, StorageError> {
		let mut condition = Condition::all().add(source_fetch_signal::Column::DoneAt.gte(cutoff_time));

		if let Some(source) = source {
			condition = condition.add(source_fetch_signal::Column::Source.eq(source));
		}

		Ok(SourceFetchSignal::find()
			.filter(condition)
			.limit(limit)
			.paginate(&self.connection, 100))
	}

	#[cfg(feature = "_informant")]
	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub(crate) async fn get_average_new_items_per_fetch(
		&self,
		source: uuid::Uuid,
		cutoff_time: OffsetDateTime,
		limit: u64,
	) -> Result<f32, StorageError> {
		use sea_orm::sea_query::Func;

		let result = SourceFetchSignal::find()
			.filter(
				Condition::all()
					.add(source_fetch_signal::Column::Source.eq(source))
					.add(source_fetch_signal::Column::DoneAt.gte(cutoff_time)),
			)
			.select_only()
			.column_as(
				Func::coalesce([source_fetch_signal::Column::NewItemsCount.avg(), Expr::value(0.0)]).into_simple_expr(),
				"average",
			)
			.limit(limit)
			.into_tuple::<f32>()
			.one(&self.connection)
			.await?;

		Ok(result.unwrap_or(0.0))
	}

	#[cfg(feature = "_informant")]
	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub(crate) async fn get_maximum_new_items_observed(
		&self,
		source: uuid::Uuid,
		limit: u64,
	) -> Result<i32, StorageError> {
		use sea_orm::sea_query::Func;

		let result = SourceFetchSignal::find()
			.filter(source_fetch_signal::Column::Source.eq(source))
			.select_only()
			.column_as(
				Func::coalesce([source_fetch_signal::Column::NewItemsCount.max(), Expr::value(0)]).into_simple_expr(),
				"maximum",
			)
			.limit(limit)
			.into_tuple::<i32>()
			.one(&self.connection)
			.await?;

		Ok(result.unwrap_or(0))
	}

	#[cfg(feature = "_informant")]
	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub(crate) async fn get_average_fetch_duration(
		&self,
		source: Option<uuid::Uuid>,
		cutoff_time: OffsetDateTime,
		limit: u64,
	) -> Result<Duration, StorageError> {
		let mut fetch_signal_paginator = self.get_source_fetch_signals(source, cutoff_time, limit).await?;

		let mut sum = Duration::ZERO;
		let mut count = 0;

		while let Some(signals) = fetch_signal_paginator.fetch_and_next().await? {
			for signal in signals {
				sum += Duration::milliseconds(signal.duration as i64);
				count += 1;
			}
		}

		if count > 0 { Ok(sum / count) } else { Ok(Duration::ZERO) }
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use time::Duration;
	use tracing_test::traced_test;

	pub async fn setup_source(con: &StorageConnection) -> Uuid {
		let root = con
			.create_source_directory(None, "Root".to_owned(), None)
			.await
			.unwrap();

		let source_id = con
			.add_source(
				root,
				crate::inform::Parameters::TestPlaceholder,
				crate::net::InterfaceType::TestPlaceholder,
				"Test Source".to_owned(),
				None,
				true,
			)
			.await
			.unwrap();

		source_id
	}

	#[cfg(feature = "_informant")]
	pub async fn setup_news_for_signals(con: &StorageConnection, source_id: Uuid) {
		con.add_news(
			source_id,
			vec![super::super::impl_news_storage::InputNews {
				source_provided_id: None,
				uri: Some("https://example.com/1".to_owned()),
				title: "News".to_owned(),
				summary: None,
				content: None,
				published_at: None,
				updated_at: None,
			}],
		)
		.await
		.unwrap();
	}

	#[cfg(feature = "_informant")]
	pub async fn setup_signals_for_news(con: &StorageConnection, news_id: Uuid, now: OffsetDateTime) {
		// Appearance signal

		con.store_user_feedback_signal(UserFeedbackSignal::NewsApearanceSignal {
			news: news_id,
			happened_at: now,
			duration: Duration::seconds(5),
		})
		.await
		.unwrap();
		con.store_user_feedback_signal(UserFeedbackSignal::NewsApearanceSignal {
			news: news_id,
			happened_at: now,
			duration: Duration::seconds(50),
		})
		.await
		.unwrap();
		con.store_user_feedback_signal(UserFeedbackSignal::NewsApearanceSignal {
			news: news_id,
			happened_at: now,
			duration: Duration::seconds(1000),
		})
		.await
		.unwrap();

		// Focus signal

		con.store_user_feedback_signal(UserFeedbackSignal::NewsFocusSignal {
			news: news_id,
			done_at: OffsetDateTime::now_utc(),
			duration: Duration::milliseconds(1500),
		})
		.await
		.unwrap();

		// Read signal

		// Valid read signals
		for percentage in [0, 50, 75, 100] {
			con.store_user_feedback_signal(UserFeedbackSignal::NewsReadSignal {
				news: news_id,
				done_at: now + Duration::seconds(percentage as i64), // Just to keep some time difference
				duration: Duration::seconds(30),
				scroll_depth_percentage: percentage,
			})
			.await
			.unwrap();
		}

		// Invalid read signals
		con.store_user_feedback_signal(UserFeedbackSignal::NewsReadSignal {
			news: news_id,
			done_at: now,
			duration: Duration::seconds(30),
			scroll_depth_percentage: -1,
		})
		.await
		.unwrap_err();
		con.store_user_feedback_signal(UserFeedbackSignal::NewsReadSignal {
			news: news_id,
			done_at: now,
			duration: Duration::seconds(30),
			scroll_depth_percentage: 101,
		})
		.await
		.unwrap_err();

		// Vote signal

		// Upvote
		con.store_user_feedback_signal(UserFeedbackSignal::NewsExplicitVoteSignal {
			news: news_id,
			done_at: now,
			is_up_vote: true,
		})
		.await
		.unwrap();
		// Downvote
		con.store_user_feedback_signal(UserFeedbackSignal::NewsExplicitVoteSignal {
			news: news_id,
			done_at: now,
			is_up_vote: false,
		})
		.await
		.unwrap();
	}

	#[cfg(feature = "_informant")]
	pub async fn setup_signals_for_source(con: &StorageConnection, source_id: Uuid, now: OffsetDateTime) {
		con.store_source_feedback_signal(SourceFeedbackSignal::FetchSignal {
			source: source_id,
			done_at: now,
			duration: Duration::seconds(5),
			failure_code: None,
			new_items_count: 10,
			latest_publish_at: Some(now - Duration::hours(3)),
			oldest_publish_at: Some(now - Duration::days(1)),
		})
		.await
		.unwrap();
		con.store_source_feedback_signal(SourceFeedbackSignal::FetchSignal {
			source: source_id,
			done_at: OffsetDateTime::now_utc(),
			duration: Duration::seconds(5),
			failure_code: Some(crate::inform::InformantErrorKind::Network),
			new_items_count: 0,
			latest_publish_at: None,
			oldest_publish_at: None,
		})
		.await
		.unwrap();
		con.store_source_feedback_signal(SourceFeedbackSignal::FetchSignal {
			source: source_id,
			done_at: OffsetDateTime::now_utc(),
			duration: Duration::seconds(1),
			failure_code: Some(crate::inform::InformantErrorKind::Network),
			new_items_count: 20,
			latest_publish_at: None,
			oldest_publish_at: None,
		})
		.await
		.unwrap();
		con.store_source_feedback_signal(SourceFeedbackSignal::FetchSignal {
			source: source_id,
			done_at: OffsetDateTime::now_utc(),
			duration: Duration::seconds(1),
			failure_code: None,
			new_items_count: 30,
			latest_publish_at: None,
			oldest_publish_at: None,
		})
		.await
		.unwrap();
	}

	#[cfg(feature = "_informant")]
	#[tokio::test]
	#[traced_test]
	async fn user_feedback_signals_logic() {
		let con = crate::db::impl_migration::tests::test_connection_and_migrations().await;
		let source_id = setup_source(&con).await;
		setup_news_for_signals(&con, source_id).await;

		let now = OffsetDateTime::now_utc();

		let news_list = con
			.get_news_with_filter(
				super::super::impl_news_storage::NewsFilter::default(),
				super::super::SortOrder::Ascending,
				super::super::impl_news_storage::TIME_MIN,
				super::super::impl_news_storage::TIME_MAX,
				100,
			)
			.await
			.unwrap();
		let news_id = news_list[0].id;

		setup_signals_for_news(&con, news_id, now).await;

		// Appearance signal

		assert_eq!(
			con.get_news_appearance_signals(news_id, now - Duration::hours(1), 100)
				.await
				.unwrap()
				.num_items()
				.await
				.unwrap(),
			3
		);
		assert_eq!(
			con.get_source_appearance_signals(source_id, now - Duration::hours(1), 100)
				.await
				.unwrap()
				.num_items()
				.await
				.unwrap(),
			3
		);

		// Focus signal

		assert_eq!(
			con.get_focus_signals(source_id, now - Duration::hours(1), 100)
				.await
				.unwrap()
				.num_items()
				.await
				.unwrap(),
			1
		);

		// Read signal

		assert_eq!(
			con.get_read_signals(source_id, now - Duration::hours(1), 100)
				.await
				.unwrap()
				.num_items()
				.await
				.unwrap(),
			4
		);

		// Vote signal

		assert_eq!(
			con.get_vote_signals(source_id, now - Duration::hours(1), 100)
				.await
				.unwrap()
				.num_items()
				.await
				.unwrap(),
			2
		);
	}

	#[cfg(feature = "_informant")]
	#[tokio::test]
	#[traced_test]
	async fn source_feedback_signals_logic() {
		let con = crate::db::impl_migration::tests::test_connection_and_migrations().await;
		let source_id = setup_source(&con).await;

		let now = OffsetDateTime::now_utc();

		setup_signals_for_source(&con, source_id, now).await;

		assert_eq!(
			con.get_source_fetch_signals(Some(source_id), now - Duration::hours(1), 100)
				.await
				.unwrap()
				.num_items()
				.await
				.unwrap(),
			4
		);
		assert_eq!(
			con.get_source_fetch_signals(None, now - Duration::hours(1), 100)
				.await
				.unwrap()
				.num_items()
				.await
				.unwrap(),
			4
		);
	}

	#[cfg(feature = "_informant")]
	#[tokio::test]
	#[traced_test]
	async fn average_new_items_per_fetch_logic() {
		let con = crate::db::impl_migration::tests::test_connection_and_migrations().await;
		let source_id = setup_source(&con).await;

		let now = OffsetDateTime::now_utc();

		let average = con.get_average_new_items_per_fetch(source_id, now, 100).await.unwrap();

		assert_eq!(average, 0.0);

		setup_signals_for_source(&con, source_id, now).await;

		let average = con
			.get_average_new_items_per_fetch(source_id, now - Duration::days(1), 100)
			.await
			.unwrap();

		assert!(average > 0.0);
		assert!(average <= 20.0);
	}

	#[cfg(feature = "_informant")]
	#[tokio::test]
	#[traced_test]
	async fn maximum_new_items_observed_logic() {
		let con = crate::db::impl_migration::tests::test_connection_and_migrations().await;
		let source_id = setup_source(&con).await;

		let now = OffsetDateTime::now_utc();

		assert_eq!(con.get_maximum_new_items_observed(source_id, 100).await.unwrap(), 0);

		setup_signals_for_source(&con, source_id, now).await;

		assert_eq!(con.get_maximum_new_items_observed(source_id, 100).await.unwrap(), 30);
	}

	#[cfg(feature = "_informant")]
	#[tokio::test]
	#[traced_test]
	async fn average_fetch_duration_logic() {
		let con = crate::db::impl_migration::tests::test_connection_and_migrations().await;
		let source_id = setup_source(&con).await;

		let now = OffsetDateTime::now_utc();

		assert_eq!(
			con.get_average_fetch_duration(Some(source_id), OffsetDateTime::now_utc(), 100)
				.await
				.unwrap(),
			Duration::ZERO
		);

		setup_signals_for_source(&con, source_id, now).await;

		assert_eq!(
			con.get_average_fetch_duration(Some(source_id), now - Duration::days(1), 100)
				.await
				.unwrap(),
			Duration::seconds(3)
		);
		assert_eq!(
			con.get_average_fetch_duration(None, now - Duration::days(1), 100)
				.await
				.unwrap(),
			Duration::seconds(3)
		);
	}
}
