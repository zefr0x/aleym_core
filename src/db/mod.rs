// TODO: Create custom structs for each output when everything is clear.
#[expect(unused)]
pub(crate) mod entities;
mod error;
mod impl_migration;
mod impl_news_recommendation;
mod impl_news_storage;
mod impl_signals_storage;
mod impl_source_storage;
mod migration;

pub use sea_orm::ActiveValue;
use sea_orm::{Database, DatabaseConnection};
pub use time;
pub use uuid;

pub use error::StorageError;
pub use impl_news_storage::*;
pub use impl_signals_storage::*;
pub use migration::Migrator;

#[derive(Clone, Debug)]
pub enum SortOrder {
	Ascending,
	Descending,
}

#[derive(Debug)]
pub enum ScheduleNotify {
	SourceEnabled(uuid::Uuid),
	SourceDisabled(uuid::Uuid),
}

#[derive(Clone, Debug)]
pub struct StorageConnection {
	connection: DatabaseConnection,
	schedule_notification_sender: Option<tokio::sync::mpsc::Sender<ScheduleNotify>>,
}

impl StorageConnection {
	/// Construct and create a new database connection pool.
	///
	/// Parent directories must exist, they will not be created when needed.
	/// If `database_file` is [`None`], a temporary in-memory database will be created.
	pub(crate) async fn new(database_file: Option<&std::path::Path>) -> Result<Self, StorageError> {
		let database_url = match database_file {
			Some(database_file) => {
				format!(
					"sqlite://{}?mode=rwc",
					std::path::absolute(database_file)?
						.to_str()
						.ok_or(StorageError::InvalidUtf8Path)?
				)
			}
			None => "sqlite::memory:".to_owned(),
		};

		tracing::info!(?database_url, "creating database connection");

		let connection = Database::connect(database_url).await?;

		Ok(Self {
			connection,
			schedule_notification_sender: None,
		})
	}

	/// Create a channel to receive schedule notifications from the storage.
	///
	/// Should be closed using [`StorageConnection::close_notifications_channel()`].
	pub fn open_notifications_channel(&mut self) -> tokio::sync::mpsc::Receiver<ScheduleNotify> {
		let (sender, receiver) = tokio::sync::mpsc::channel::<ScheduleNotify>(1);

		self.schedule_notification_sender = Some(sender);

		receiver
	}

	/// Close the sender of the schedule notifications channel.
	///
	/// This should be used first instead of closing the receiver to prevent unnecessary attempts by the storage to send.
	pub fn close_notifications_channel(&mut self) {
		self.schedule_notification_sender = None;
	}

	pub(crate) async fn send_scheduler_notification(&self, notification: ScheduleNotify) {
		if let Some(events_sender) = &self.schedule_notification_sender {
			tracing::debug!(?notification, "sending to scheduler notification channel");

			if let Err(error) = events_sender.send(notification).await {
				tracing::error!(
					event=?error.0,
					"Failed to send notification due to the scheduler's receiver being closed"
				);
			}
		}
	}
}
