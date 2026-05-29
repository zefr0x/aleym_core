use time::Duration;

use crate::db;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
	/// How much to prefer recent user feedback against old ones.
	/// Ranges between `0.0` and `1.0`.
	pub feedback_freshness_bias: f32,

	/// Cut-off time of appearance signals to be included in calculations.
	pub source_appearance_cutoff: Duration,
	/// Cut-off count of appearance signals to be included in calculations.
	pub source_appearance_limit: u64,

	/// Cut-off time of appearance signals to be included in calculations.
	pub news_appearance_cutoff: Duration,
	/// Cut-off count of appearance signals to be included in calculations.
	pub news_appearance_limit: u64,

	/// Cut-off time of focus signals to be included in calculations.
	pub focus_signals_cutoff: Duration,
	/// Cut-off count of focus signals to be included in calculations.
	pub focus_signals_limit: u64,
	/// How much to prefer focus signals against read and vote when scoring recommended news items.
	/// Ranges between `0.0` and `1.0`.
	pub focus_score_weight: f32,

	/// Cut-off time of read signals to be included in calculations.
	pub read_signals_cutoff: Duration,
	/// Cut-off count of read signals to be included in calculations.
	pub read_signals_limit: u64,
	/// How much to prefer read signals against focus and vote when scoring recommended news items.
	/// Ranges between `0.0` and `1.0`.
	pub read_score_weight: f32,

	/// Cut-off time of vote signals to be included in calculations.
	pub vote_signals_cutoff: Duration,
	/// Cut-off count of vote signals to be included in calculations.
	pub vote_signals_limit: u64,
	/// How much to prefer vote signals against focus and read when scoring recommended news items.
	/// Ranges between `0.0` and `1.0`.
	pub vote_score_weight: f32,
}

impl Default for Config {
	fn default() -> Self {
		// TODO: Study the best defaults and select better values.
		Self {
			feedback_freshness_bias: 0.35,
			source_appearance_cutoff: Duration::minutes(30),
			source_appearance_limit: 1000,
			news_appearance_cutoff: Duration::hours(12),
			news_appearance_limit: 1000,
			focus_signals_cutoff: Duration::days(30),
			focus_signals_limit: 1000,
			focus_score_weight: 0.15,
			read_signals_cutoff: Duration::days(30),
			read_signals_limit: 1000,
			read_score_weight: 0.45,
			vote_signals_cutoff: Duration::days(30),
			vote_signals_limit: 1000,
			vote_score_weight: 0.5,
		}
	}
}

pub(crate) struct RecommendationWeighter {
	config: Config,
}

impl RecommendationWeighter {
	pub(crate) fn new(config: Config) -> Self {
		Self { config }
	}

	/// Returns value between `0.0` and `1.0`, where `0.0` means that it is fully suppressed.
	#[tracing::instrument(skip(self, appearance_signals_paginator), level = tracing::Level::DEBUG)]
	async fn calculate_source_appearance_suppression<'db>(
		&self,
		source_id: uuid::Uuid,
		now: time::OffsetDateTime,
		mut appearance_signals_paginator: sea_orm::Paginator<
			'db,
			sea_orm::DatabaseConnection,
			sea_orm::SelectModel<db::entities::news_apearance_signal::Model>,
		>,
	) -> Result<f32, db::StorageError> {
		let mut sum = 0.0f32;

		// NOTE: Source appearance decay should be faster than news appearance decay.
		while let Some(signals) = appearance_signals_paginator.fetch_and_next().await? {
			for signal in signals {
				let time_since_read = (now - signal.happened_at).as_seconds_f32();

				let appearance_duration = signal.duration as f32; // milliseconds

				// TODO: Make the rate configurable.
				sum += time_since_read * 1000.0 / appearance_duration;
			}
		}

		// Exponential Decay
		let suppression_factor = if sum > 0.0 {
			1.0 - (-sum * self.config.feedback_freshness_bias).exp()
		} else {
			1.0
		};

		Ok(suppression_factor)
	}

	/// Returns value between `0.0` and `1.0`, where `0.0` means that it is fully suppressed.
	#[tracing::instrument(skip(self, appearance_signals_paginator), level = tracing::Level::DEBUG)]
	pub(crate) async fn calculate_news_appearance_suppression<'db>(
		&self,
		source_id: uuid::Uuid,
		now: time::OffsetDateTime,
		mut appearance_signals_paginator: sea_orm::Paginator<
			'db,
			sea_orm::DatabaseConnection,
			sea_orm::SelectModel<db::entities::news_apearance_signal::Model>,
		>,
	) -> Result<f32, db::StorageError> {
		let mut sum = 0.0f32;

		// NOTE: News appearance decay should be slower than source appearance decay.
		while let Some(signals) = appearance_signals_paginator.fetch_and_next().await? {
			for signal in signals {
				let time_since_read = (now - signal.happened_at).whole_minutes() as f32;

				let appearance_duration = signal.duration as f32; // milliseconds

				// TODO: Make the rate configurable.
				sum += time_since_read * 1000.0 / appearance_duration;
			}
		}

		// Exponential Decay
		let suppression_factor = if sum > 0.0 {
			1.0 - (-sum * self.config.feedback_freshness_bias).exp()
		} else {
			1.0
		};

		Ok(suppression_factor)
	}

	/// Returns value between `0.0` and `1.0`, where `0.0` means no possibility for recommendation, and `1.0` is the
	/// likeliest of being recommended.
	#[tracing::instrument(
		skip(self, focus_signals_paginator, read_signals_paginator, vote_signals_paginator, appearance_signals_paginator),
		level = tracing::Level::DEBUG
	)]
	pub(crate) async fn calculate_source_score<'db>(
		&self,
		source: uuid::Uuid,
		now: time::OffsetDateTime,
		mut focus_signals_paginator: sea_orm::Paginator<
			'db,
			sea_orm::DatabaseConnection,
			sea_orm::SelectModel<db::entities::news_focus_signal::Model>,
		>,
		mut read_signals_paginator: sea_orm::Paginator<
			'db,
			sea_orm::DatabaseConnection,
			sea_orm::SelectModel<db::entities::news_read_signal::Model>,
		>,
		mut vote_signals_paginator: sea_orm::Paginator<
			'db,
			sea_orm::DatabaseConnection,
			sea_orm::SelectModel<db::entities::news_explicit_vote_signal::Model>,
		>,
		appearance_signals_paginator: sea_orm::Paginator<
			'db,
			sea_orm::DatabaseConnection,
			sea_orm::SelectModel<db::entities::news_apearance_signal::Model>,
		>,
	) -> Result<f32, db::StorageError> {
		let mut focus_sum = 0.0f32;

		while let Some(signals) = focus_signals_paginator.fetch_and_next().await? {
			for signal in signals {
				let time_since = (now - signal.done_at).whole_minutes() as f32;

				let focus_duration = signal.duration as f32 / 1000.0; // Seconds

				// TODO: Find the optimal rate or make it configurable.
				focus_sum += time_since / focus_duration;
			}
		}

		let mut read_sum = 0.0f32;

		while let Some(signals) = read_signals_paginator.fetch_and_next().await? {
			for signal in signals {
				let time_since = (now - signal.done_at).whole_hours() as f32;

				let scroll_depth = signal.scroll_depth_percentage as f32 / 100.0;
				let read_duration = signal.duration as f32 / 1000.0; // Seconds

				// TODO: Find the optimal rate or make it configurable.
				read_sum += time_since / (read_duration * scroll_depth);
			}
		}

		let mut vote_sum = 0.0f32;

		while let Some(signals) = vote_signals_paginator.fetch_and_next().await? {
			for signal in signals {
				let time_since = (now - signal.done_at).whole_days() as f32;

				if signal.is_up_vote {
					vote_sum += time_since;
				} else {
					vote_sum -= time_since;
				}
			}
		}
		// TODO: Handle down votes properly.
		if vote_sum < 0.0 {
			vote_sum = 0.0;
		}

		// Exponential Decay
		let score = if focus_sum > 0.0 || read_sum > 0.0 || vote_sum > 0.0 {
			1.0 - ((-focus_sum * self.config.feedback_freshness_bias).exp() * self.config.focus_score_weight
				+ (-read_sum * self.config.feedback_freshness_bias).exp() * self.config.read_score_weight
				+ (-vote_sum * self.config.feedback_freshness_bias).exp() * self.config.vote_score_weight)
				.clamp(0.0, 1.0)
		} else {
			1.0
		};

		let source_apperance_suppression = self
			.calculate_source_appearance_suppression(source, now, appearance_signals_paginator)
			.await?;

		let suppressed_score = score * source_apperance_suppression;

		tracing::trace!(
			focus_sum,
			read_sum,
			vote_sum,
			source_apperance_suppression,
			score,
			suppressed_score,
			"Calculated final source score"
		);

		Ok(suppressed_score)
	}
}
