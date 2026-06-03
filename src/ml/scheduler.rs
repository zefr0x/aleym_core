use std::collections::BTreeMap;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use super::SchedulerError;
use crate::db::{self, StorageError};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
	/// Intervals will never be reduce below this (rate limit).
	pub min_fetch_interval: Duration,

	/// Intervals will never be increased beyond this, all sources will be fetched at least once during this window.
	pub max_fetch_interval: Duration,

	/// Short time window for analyzing signals.
	pub short_term_cutoff_time: Duration,

	/// Long time window for analyzing signals.
	pub long_term_cutoff_time: Duration,

	/// How much to prefer recent feedback signals against old ones.
	/// Ranges between `0.0` and `1.0`.
	pub fetch_freshness_bias: f32,

	/// Cut-off count of signals to be included in calculations.
	pub signals_count_limit: u64,

	/// Threshold new items count to consider a signal in average publication window calculation.
	pub publication_window_new_items_count_threshold: i32,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			min_fetch_interval: Duration::minutes(5),
			max_fetch_interval: Duration::hours(12),
			short_term_cutoff_time: Duration::days(1),
			long_term_cutoff_time: Duration::days(30),
			fetch_freshness_bias: 0.2,
			signals_count_limit: 1000,
			publication_window_new_items_count_threshold: 15,
		}
	}
}

#[derive(Debug)]
pub(crate) struct Calender {
	/// Map of scheduled times to source fetches (may have multiple sources with the same time)
	calendar: BTreeMap<OffsetDateTime, Vec<Uuid>>,
	config: Config,
}

impl Calender {
	pub fn new(config: Config) -> Self {
		Self {
			calendar: BTreeMap::new(),
			config,
		}
	}

	/// Get the next scheduled time, or [`None`] if calendar is empty.
	pub(crate) fn next_time(&self) -> Option<OffsetDateTime> {
		self.calendar.keys().next().copied()
	}

	/// Get all sources scheduled at or before `until_time` and remove them from calendar.
	#[tracing::instrument(skip(self), level = tracing::Level::TRACE)]
	pub fn pop_due(&mut self, until_time: OffsetDateTime) -> Vec<Uuid> {
		let mut due_tasks = vec![];

		let keys_to_remove = self
			.calendar
			.range(..=until_time)
			.map(|(k, _)| *k)
			.collect::<Vec<OffsetDateTime>>();

		for key in keys_to_remove {
			if let Some(mut tasks) = self.calendar.remove(&key) {
				due_tasks.append(&mut tasks);
			}
		}

		due_tasks
	}

	async fn calculate_interval_bounds(
		&self,
		source: uuid::Uuid,
		storage: &db::StorageConnection,
		now: OffsetDateTime,
	) -> Result<(Duration, Duration), StorageError> {
		let recent_average_new_items = storage
			.get_average_new_items_per_fetch(
				source,
				now - self.config.short_term_cutoff_time,
				self.config.signals_count_limit,
			)
			.await?;
		let average_new_items = storage
			.get_average_new_items_per_fetch(
				source,
				now - self.config.long_term_cutoff_time,
				self.config.signals_count_limit,
			)
			.await?;

		let max_new_items = storage
			.get_maximum_new_items_observed(source, self.config.signals_count_limit)
			.await?;

		let average_source_fetch_duration = storage
			.get_average_fetch_duration(
				Some(source),
				now - self.config.long_term_cutoff_time,
				self.config.signals_count_limit,
			)
			.await?;
		let average_fetch_duration = storage
			.get_average_fetch_duration(
				None,
				now - self.config.long_term_cutoff_time,
				self.config.signals_count_limit,
			)
			.await?;

		let mut source_fetsh_signals = storage
			.get_source_fetch_signals(
				Some(source),
				now - self.config.long_term_cutoff_time,
				self.config.signals_count_limit,
			)
			.await?;

		// Calculate success rate and average publication window.

		let mut weighted_success_count = 0.0;
		let mut total_success_weight = 0.0;

		let mut weighted_publication_sum = Duration::ZERO;
		let mut total_publication_weight = 0.0;

		while let Some(signals) = source_fetsh_signals.fetch_and_next().await? {
			for signal in signals {
				let time_since = (now - signal.done_at).whole_minutes() as f32;

				// Success rate

				let weight = (-time_since * self.config.fetch_freshness_bias).exp();

				if signal.failure_code.is_none() {
					weighted_success_count += weight;
				}
				total_success_weight += weight;

				// Predicated publication window

				if let (Some(latest_publish_at), Some(oldest_publish_at)) =
					(signal.latest_publish_at, signal.oldest_publish_at)
					&& latest_publish_at != oldest_publish_at
					&& signal.new_items_count >= self.config.publication_window_new_items_count_threshold
				{
					let publication_window = latest_publish_at - oldest_publish_at;

					// Linear decay
					let time_since_minutes = (now - signal.done_at).whole_minutes() as f32;
					let age_multiplier = 1.0 + (time_since_minutes * self.config.fetch_freshness_bias / 1440.0);
					let decay_weight = 1.0 / age_multiplier;

					let adjusted_window_secs = publication_window * age_multiplier;

					weighted_publication_sum += adjusted_window_secs * decay_weight;
					total_publication_weight += decay_weight;
				}
			}
		}

		let fetch_success_rate = if total_success_weight > 0.0 {
			weighted_success_count / total_success_weight
		} else {
			1.0
		};

		let predicated_average_source_publication_window = if total_publication_weight > 0.0 {
			weighted_publication_sum / total_publication_weight
		} else {
			Duration::MAX
		};

		// Calculate intervals

		let mut min_interval = self.config.min_fetch_interval;
		let mut max_interval = self.config.max_fetch_interval;
		let base_time_window = max_interval.saturating_sub(min_interval);

		tracing::debug!(base_min_interval=?min_interval, base_max_interval=?max_interval, "starting interval calculation");

		// Error rate impact
		min_interval += base_time_window * (1.0 - fetch_success_rate) * 0.2;

		// Duration of fetching impact
		if average_fetch_duration > Duration::ZERO {
			min_interval +=
				base_time_window * (average_source_fetch_duration / average_fetch_duration).clamp(0.0, 1.0) * 0.125;
		}

		// Reaching the possible maximum news items, so decrease minimum and maximum
		if max_new_items > 0 {
			let max_reaching_rate = average_new_items / max_new_items as f32;
			let recent_max_reaching_rate = recent_average_new_items / max_new_items as f32;

			let max_reaching_duration = base_time_window * max_reaching_rate;
			let recent_max_reaching_duration = base_time_window * recent_max_reaching_rate;

			min_interval -= max_reaching_duration * 0.1 + recent_max_reaching_duration * 0.33;
			max_interval -= max_reaching_duration * 0.33 + recent_max_reaching_duration * 0.7;
		}

		// TODO: Investigate if range saturation may happen here.
		// If predicated average publication window is very long compared to the base window, than decrease maximum
		max_interval -= base_time_window
			* (1.0
				- predicated_average_source_publication_window.min(max_interval - min_interval)
					/ (max_interval - min_interval))
				.clamp(0.0, 1.0)
			* 0.3;

		// Enforce limits
		if min_interval < self.config.min_fetch_interval {
			min_interval = self.config.min_fetch_interval;
		}
		if max_interval > self.config.max_fetch_interval {
			max_interval = self.config.max_fetch_interval;
		}
		if min_interval > self.config.max_fetch_interval {
			min_interval = self.config.max_fetch_interval;
		}
		if max_interval < self.config.min_fetch_interval {
			max_interval = self.config.min_fetch_interval;
		}

		tracing::debug!(?min_interval, ?max_interval, "calculated interval bounds");

		Ok((min_interval, max_interval))
	}

	#[tracing::instrument(skip(self, rng), level = tracing::Level::DEBUG)]
	pub(crate) async fn schedule_source(
		&mut self,
		source: Uuid,
		rng: &mut impl rand::Rng,
		storage: &db::StorageConnection,
	) -> Result<(), db::StorageError> {
		use rand::RngExt as _;

		let now = OffsetDateTime::now_utc();

		let (min_interval, max_interval) = self.calculate_interval_bounds(source, storage, now).await?;

		// Pick a random time between min_interval and max_interval

		let time_window = max_interval.saturating_sub(min_interval).whole_milliseconds();

		let random_offset = if time_window > 0 {
			rng.random_range(0..=time_window)
		} else {
			0
		};

		let scheduled_time = now + self.config.min_fetch_interval + Duration::milliseconds(random_offset.try_into()?);

		self.calendar.entry(scheduled_time).or_default().push(source);

		Ok(())
	}

	/// Remove a scheduled source from the calendar.
	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub(crate) fn unschedule_fetch(&mut self, source: Uuid) -> Result<(), SchedulerError> {
		let entry = self
			.calendar
			.iter()
			.find_map(|entry| entry.1.iter().position(|x| x == &source).map(|index| (*entry.0, index)))
			.ok_or(SchedulerError::SourceNotScheduled(source))?;

		self.calendar
			.get_mut(&entry.0)
			.ok_or(SchedulerError::SourceNotScheduled(source))?
			.remove(entry.1);

		if self.calendar.get(&entry.0).unwrap().is_empty() {
			self.calendar.remove(&entry.0).unwrap();
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use time::OffsetDateTime;

	#[test]
	fn calender_logic() {
		let config = Config::default();

		let mut calendar = Calender::new(config);

		assert!(calendar.next_time().is_none());
		assert!(calendar.pop_due(OffsetDateTime::now_utc()).is_empty());

		let now = OffsetDateTime::now_utc();

		let uuid1 = Uuid::new_v4();
		let uuid2 = Uuid::new_v4();
		let uuid3 = Uuid::new_v4();

		calendar.calendar.insert(now + Duration::hours(2), vec![uuid1]);
		calendar.calendar.insert(now + Duration::hours(1), vec![uuid2]);

		assert_eq!(calendar.next_time(), Some(now + Duration::hours(1)));

		let due = calendar.pop_due(now + Duration::minutes(65));

		assert_eq!(due.len(), 1);
		assert!(due.contains(&uuid2));
		assert!(!calendar.calendar.is_empty());

		let result = calendar.unschedule_fetch(uuid3);
		assert!(result.is_err());
		match result {
			Err(SchedulerError::SourceNotScheduled(id)) => assert_eq!(id, uuid3),
			_ => panic!("Expected SourceNotScheduled"),
		}

		calendar.unschedule_fetch(uuid1).unwrap();
		assert!(calendar.calendar.is_empty());
	}
}
