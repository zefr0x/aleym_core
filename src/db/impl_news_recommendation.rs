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
	#[tracing::instrument(skip(self, ml_config), level = tracing::Level::DEBUG)]
	pub async fn get_news_recommendations(
		&self,
		limit: u64,
		candidates_limit: u64,
		ml_config: &ml::recommendation::Config,
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
			.sample_weighted(&mut rand::rng(), limit as usize, |item| item.2)?
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
