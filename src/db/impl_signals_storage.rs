use sea_orm::{ActiveValue::Set, prelude::*};
use time::{Duration, OffsetDateTime};

#[cfg(feature = "_informant")]
use super::entities::source_fetch_signal;
use super::{
	StorageConnection, StorageError,
	entities::{news_apearance_signal, news_explicit_vote_signal, news_focus_signal, news_read_signal},
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
}
