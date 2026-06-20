use std::collections::HashMap;

use rand::seq::IndexedRandom;
use sea_orm::{Condition, QueryOrder, QuerySelect, prelude::*};

use super::{StorageConnection, StorageError, entities::news};
use crate::ml;

impl StorageConnection {
	/// Yields different results on each execution based on feedback signals and some randomness. While results may overlap
	/// when the `IsRead` status of news hasn't changed, feedback signals can reduce this possibility, delaying it from
	/// happening between consecutive executions to improve user experience.
	///
	/// There is no cursor, so the list should be fixed in size without paging or infinite scrolling.
	///
	/// Shouldn't be used frequently to update the list when the user may be focusing on it.
	///
	/// `candidates_limit` is the bottleneck, it should be large enough for better and diverse recommendations.
	///
	///
	/// [`rand::Rng`] implementation provide randomness for selecting weighted candidate items for recoemmendation,
	/// basic pseudo-random number generators (PRNGs) are suitable for this case.
	#[tracing::instrument(skip(self, ml_config, rng), level = tracing::Level::DEBUG)]
	pub async fn get_news_recommendations(
		&self,
		limit: u64,
		candidates_limit: u64,
		ml_config: &ml::recommendation::Config,
		mut rng: &mut impl rand::Rng,
	) -> Result<Vec<news::Model>, StorageError> {
		// First, get candidates

		let candidates = news::Entity::find()
			.filter(
				Condition::all()
					.add(news::Column::IsRead.eq(false))
					.add(news::Column::IsLatestVersion.eq(true)),
			)
			// TODO: Should we select candidates randomly rather than from latest publications?
			.order_by_desc(news::Column::PublishedAt)
			.order_by_desc(news::Column::FirstFetchedAt)
			.limit(candidates_limit)
			.all(&self.connection)
			.await?;

		if candidates.is_empty() {
			tracing::debug!("No news candidates found for recommendation");
			return Ok(vec![]);
		}

		// Second, weight candidates

		let mut source_scores: HashMap<Uuid, f32> = HashMap::new();
		let mut weighted_candidates = vec![];

		for (index, item) in candidates.into_iter().enumerate() {
			let scorer = crate::ml::recommendation::RecommendationWeighter::new(ml_config.clone());

			let now = time::OffsetDateTime::now_utc();

			let source_score = if let Some(score) = source_scores.get(&item.source) {
				*score
			} else {
				let focus_signals = self
					.get_focus_signals(
						item.source,
						now - ml_config.focus_signals_cutoff,
						ml_config.focus_signals_limit,
					)
					.await?;
				let read_signals = self
					.get_read_signals(
						item.source,
						now - ml_config.read_signals_cutoff,
						ml_config.read_signals_limit,
					)
					.await?;
				let vote_signals = self
					.get_vote_signals(
						item.source,
						now - ml_config.vote_signals_cutoff,
						ml_config.vote_signals_limit,
					)
					.await?;
				let source_apperance_signals = self
					.get_source_appearance_signals(
						item.source,
						now - ml_config.source_appearance_cutoff,
						ml_config.source_appearance_limit,
					)
					.await?;

				let score = scorer
					.calculate_source_score(
						item.source,
						now,
						focus_signals,
						read_signals,
						vote_signals,
						source_apperance_signals,
					)
					.await?;

				source_scores.insert(item.source, score);

				score
			};

			let news_apperance_signals = self
				.get_news_appearance_signals(
					item.id,
					now - ml_config.news_appearance_cutoff,
					ml_config.news_appearance_limit,
				)
				.await?;

			let news_suppression_factor = scorer
				.calculate_news_appearance_suppression(item.id, now, news_apperance_signals)
				.await?;

			tracing::trace!(
				source.id=?item.source,
				source.score=source_score,
				news.id=?item.id,
				news.apperance_suppression_factor=news_suppression_factor
			);

			weighted_candidates.push((index, item, (source_score * news_suppression_factor)));
		}

		// Finally, sample with weighted random

		// PERF: This is messy due to `sample_weighted()` returning references, there is no owned value return variant.

		let sample_indexes = weighted_candidates
			.sample_weighted(&mut rng, limit as usize, |item| item.2)?
			.map(|item| item.0)
			.collect::<Vec<usize>>();

		Ok(weighted_candidates
			.into_iter()
			.filter_map(|(index, item, _)| {
				if sample_indexes.contains(&index) {
					Some(item)
				} else {
					None
				}
			})
			.collect())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use time::{Duration, OffsetDateTime};
	use tracing_test::traced_test;

	#[cfg(feature = "_informant")]
	pub async fn setup_news_items(con: &StorageConnection, now: OffsetDateTime) {
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

		let items = vec![
			super::super::impl_news_storage::InputNews {
				source_provided_id: None,
				uri: Some("https://example.com/1".to_owned()),
				title: "News 1".to_owned(),
				summary: None,
				content: None,
				published_at: None,
				updated_at: None,
			},
			super::super::impl_news_storage::InputNews {
				source_provided_id: None,
				uri: Some("https://example.com/2".to_owned()),
				title: "News 2".to_owned(),
				summary: None,
				content: None,
				published_at: None,
				updated_at: None,
			},
			super::super::impl_news_storage::InputNews {
				source_provided_id: None,
				uri: Some("https://example.com/3".to_owned()),
				title: "News 3".to_owned(),
				summary: None,
				content: None,
				published_at: None,
				updated_at: None,
			},
		];
		let news_output = con.add_news(source_id, items).await.unwrap();
		let news1_id = news_output.new[0];
		let news2_id = news_output.new[1];
		let news3_id = news_output.new[2];

		// Create feedback signals

		con.store_user_feedback_signal(crate::db::UserFeedbackSignal::NewsApearanceSignal {
			news: news1_id,
			happened_at: now - Duration::seconds(500),
			duration: Duration::minutes(2),
		})
		.await
		.unwrap();
		con.store_user_feedback_signal(crate::db::UserFeedbackSignal::NewsApearanceSignal {
			news: news3_id,
			happened_at: now - Duration::seconds(500),
			duration: Duration::minutes(2),
		})
		.await
		.unwrap();
		con.store_user_feedback_signal(crate::db::UserFeedbackSignal::NewsApearanceSignal {
			news: news1_id,
			happened_at: now - Duration::seconds(100),
			duration: Duration::minutes(1),
		})
		.await
		.unwrap();
		con.store_user_feedback_signal(crate::db::UserFeedbackSignal::NewsApearanceSignal {
			news: news2_id,
			happened_at: now - Duration::seconds(101),
			duration: Duration::ZERO,
		})
		.await
		.unwrap();
		con.store_user_feedback_signal(crate::db::UserFeedbackSignal::NewsApearanceSignal {
			news: news2_id,
			happened_at: now - Duration::seconds(100),
			duration: Duration::minutes(1),
		})
		.await
		.unwrap();

		con.store_user_feedback_signal(crate::db::UserFeedbackSignal::NewsFocusSignal {
			news: news1_id,
			done_at: now - Duration::seconds(450),
			duration: Duration::seconds(3),
		})
		.await
		.unwrap();
		con.store_user_feedback_signal(crate::db::UserFeedbackSignal::NewsFocusSignal {
			news: news2_id,
			done_at: now - Duration::seconds(71),
			duration: Duration::ZERO,
		})
		.await
		.unwrap();
		con.store_user_feedback_signal(crate::db::UserFeedbackSignal::NewsFocusSignal {
			news: news2_id,
			done_at: now - Duration::seconds(70),
			duration: Duration::seconds(2),
		})
		.await
		.unwrap();

		con.store_user_feedback_signal(crate::db::UserFeedbackSignal::NewsReadSignal {
			news: news1_id,
			done_at: now - Duration::seconds(447),
			duration: Duration::seconds(25),
			scroll_depth_percentage: 100,
		})
		.await
		.unwrap();
		con.store_user_feedback_signal(crate::db::UserFeedbackSignal::NewsReadSignal {
			news: news1_id,
			done_at: OffsetDateTime::UNIX_EPOCH,
			duration: Duration::ZERO,
			scroll_depth_percentage: 0,
		})
		.await
		.unwrap();
		con.set_news_read(vec![news1_id], true).await.unwrap();

		con.store_user_feedback_signal(crate::db::UserFeedbackSignal::NewsExplicitVoteSignal {
			news: news2_id,
			done_at: now - Duration::seconds(67),
			is_up_vote: true,
		})
		.await
		.unwrap();
		con.store_user_feedback_signal(crate::db::UserFeedbackSignal::NewsExplicitVoteSignal {
			news: news2_id,
			done_at: now - Duration::seconds(67),
			is_up_vote: true,
		})
		.await
		.unwrap();
	}

	#[cfg(feature = "_informant")]
	#[tokio::test]
	#[traced_test]
	async fn news_recommendations_logic() {
		let con = crate::db::impl_migration::tests::test_connection_and_migrations().await;

		let invalid_config = crate::ml::recommendation::Config {
			focus_score_weight: 0.3,
			read_score_weight: 0.5,
			vote_score_weight: 0.3,
			..Default::default()
		};
		let config = crate::ml::recommendation::Config::default();

		assert!(
			con.get_news_recommendations(5, 15, &config, &mut rand::rng())
				.await
				.unwrap()
				.is_empty()
		);

		setup_news_items(&con, OffsetDateTime::now_utc()).await;

		con.get_news_recommendations(5, 15, &invalid_config, &mut rand::rng())
			.await
			.unwrap_err();

		let recommendations = con
			.get_news_recommendations(10, 30, &config, &mut rand::rng())
			.await
			.unwrap();
		assert!(!recommendations.is_empty());
		assert!(recommendations.len() <= 3);
		for news in recommendations {
			assert!(!news.is_read);
			assert!(news.is_latest_version);
		}
	}
}
