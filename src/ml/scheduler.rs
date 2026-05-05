use std::collections::BTreeMap;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use super::SchedulerError;
use crate::db;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
	/// Intervals will never be reduce below this (rate limit).
	pub min_fetch_interval: Duration,

	/// Intervals will never be increased beyond this, all sources will be fetched at least once during this window.
	pub max_fetch_interval: Duration,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			min_fetch_interval: Duration::minutes(5),
			max_fetch_interval: Duration::hours(12),
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

	#[tracing::instrument(skip(self, rng), level = tracing::Level::DEBUG)]
	pub(crate) async fn schedule_source(
		&mut self,
		source: Uuid,
		rng: &mut impl rand::Rng,
	) -> Result<(), db::StorageError> {
		use rand::RngExt as _;

		let now = time::OffsetDateTime::now_utc();

		// Pick a random time between min_interval and max_interval

		let time_window = self
			.config
			.max_fetch_interval
			.saturating_sub(self.config.min_fetch_interval)
			.whole_milliseconds();

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

		Ok(())
	}
}
