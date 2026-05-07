use super::{
	Error,
	db::{self, AddingNewsOutput, InputNews, SourceFeedbackSignal, StorageError, uuid::Uuid},
	inform::{self, InformantError, InformantTrait as _},
	ml,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Event {
	/// Indicates that a source has been fetched.
	NewsUpdated { source_id: Uuid, updates: AddingNewsOutput },
	/// Error occurred while executing an informant.
	InformantError { source_id: Uuid, error: String },
}

impl super::Representative {
	/// Manually execute specific informant using a specific network.
	async fn trigger_informant(
		&self,
		network: super::net::InterfaceType,
		informant_parameters: inform::Parameters,
	) -> Result<Vec<InputNews>, InformantError> {
		#[cfg(any(feature = "informant_feedrs", feature = "informant_telegram_web"))]
		let network = self.network.new_client(network);

		match informant_parameters {
			#[cfg(any(test, not(feature = "_informant")))]
			inform::Parameters::TestPlaceholder => {
				unimplemented!()
			}
			#[cfg(feature = "informant_feedrs")]
			inform::Parameters::FeedRs(parameters) => Ok(inform::feedrs::Informant::new(network).execute(parameters).await?),
			#[cfg(feature = "informant_telegram_web")]
			inform::Parameters::TelegramWeb(parameters) => Ok(inform::telegram_web::Informant::new(network)
				.execute(parameters)
				.await?),
		}
	}

	// TODO: Replace this to ask the scheduler for preference rather than forcing a manual trigger that bypasses it.
	/// Manually execute fetch operation of a specific source.
	///
	/// Note that this will not result in any machine-learning signals.
	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn trigger_informant_by_source(&self, source: Uuid) -> Result<AddingNewsOutput, Error> {
		let source = self.storage.get_source(source).await?;

		let items = self
			.trigger_informant(
				super::net::InterfaceType::try_from(source.network)?,
				serde_json::from_value::<inform::Parameters>(source.informant_parameters)
					.map_err(inform::InformantError::from)?,
			)
			.await?;

		Ok(self.storage.add_news(source.id, items).await?)
	}

	/// Manually execute fetch operation of a specific source and store signal of the results.
	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	async fn trigger_informant_with_telemetry_by_source(&self, source: Uuid) -> Result<(), Error> {
		let source = self.storage.get_source(source).await?;

		let started_at = time::OffsetDateTime::now_utc();
		let input_items = self
			.trigger_informant(
				super::net::InterfaceType::try_from(source.network)?,
				serde_json::from_value::<inform::Parameters>(source.informant_parameters)
					.map_err(inform::InformantError::from)?,
			)
			.await;

		match (input_items, time::OffsetDateTime::now_utc() - started_at) {
			(Ok(input_items), duration) => {
				let addition_output = self.storage.add_news(source.id, input_items).await?;

				self.storage
					.store_source_feedback_signal(SourceFeedbackSignal::FetchSignal {
						source: source.id,
						done_at: started_at,
						duration,
						failure_code: None,
						new_items_count: addition_output.new.len().try_into().map_err(StorageError::from)?,
						latest_publish_at: addition_output.latest_publish,
						oldest_publish_at: addition_output.oldest_publish,
					})
					.await?;

				self.send_event(move || Event::NewsUpdated {
					source_id: source.id,
					updates: addition_output,
				});
			}
			(Err(error), duration) => {
				tracing::error!(?source.id, ?error, "Informant execution failure");

				self.storage
					.store_source_feedback_signal(SourceFeedbackSignal::FetchSignal {
						source: source.id,
						done_at: started_at,
						duration,
						failure_code: Some(error.kind()),
						new_items_count: 0,
						oldest_publish_at: None,
						latest_publish_at: None,
					})
					.await?;

				self.send_event(move || Event::InformantError {
					source_id: source.id,
					error: error.to_string(),
				});
			}
		}

		Ok(())
	}

	/// This never returns unless fatal error occurs.
	///
	/// For the `notifications_receiver`, you need to get it from [`db::StorageConnection::open_notifications_channel()`] or provide your own.
	pub async fn start_scheduler(
		&self,
		mut storage_notify_receiver: tokio::sync::mpsc::Receiver<db::ScheduleNotify>,
		ml_config: ml::scheduler::Config,
	) -> Result<(), Error> {
		let mut scheduler = ml::scheduler::Calender::new(ml_config);
		let mut rng = rand::rng();

		let notify_new_enabled_source = tokio::sync::Notify::new();

		let (run_task_sender, mut run_task_receiver) = tokio::sync::mpsc::unbounded_channel::<Vec<Uuid>>();
		let (end_task_sender, mut end_task_receiver) = tokio::sync::mpsc::unbounded_channel::<Vec<Uuid>>();

		{
			let initial_enabled_sources = self
				.storage
				.get_all_sources(Some(true))
				.await?
				.into_iter()
				.map(|s| s.id)
				.collect::<Vec<Uuid>>();

			// Initialize calendar with all enabled sources
			end_task_sender.send(initial_enabled_sources).unwrap();
		}

		// Main loops
		tokio::select! {
			result = async {
				loop {
					let tasks = run_task_receiver.recv().await.unwrap();

					// TODO: Execute them concurrently (will use more resources, but will be faster).
					for source in &tasks {
						self.trigger_informant_with_telemetry_by_source(*source).await?;
					}

					end_task_sender.send(tasks).unwrap();
				}

				#[expect(unused, reason = "for type inference")]
				Ok::<(), Error>(())
			} => { result? },
			result = async {
				loop {
					tracing::debug!(?scheduler);

					tokio::select! {
						Some(notification) = storage_notify_receiver.recv() => {
							match notification {
								db::ScheduleNotify::SourceEnabled(source) => {
									tracing::debug!(source.id=?source, "scheduling newly enabled source");
									scheduler.schedule_source(source, &mut rng).await?;
									notify_new_enabled_source.notify_one();
								}
								db::ScheduleNotify::SourceDisabled(source) => {
									tracing::debug!(source.id=?source, "unscheduling disabled source");
									scheduler.unschedule_fetch(source)?;
								}
							}
						},
						Some(tasks) = end_task_receiver.recv() => {
							// NOTE: This is also used to initiate the calendar with enabled sources at the start.
							// Reschedule source
							for source in tasks {
								scheduler.schedule_source(source, &mut rng).await?;
							}
							// NOTE: We need to reset the timer since new task might got scheduled earlier.
						},
						result = async {
							let now = time::OffsetDateTime::now_utc();

							match scheduler.next_time() {
								Some(next_time) => {
									// Sleep until next task
									let sleep_duration = next_time - now;
									if sleep_duration > time::Duration::ZERO {
										// FIX: Make this sleep respect system suspend.
										tokio::time::sleep(std::time::Duration::from_millis(sleep_duration.unsigned_abs().as_millis() as u64))
											.await;
									}


									// FIX: This list might be lost when killed by ScheduleNotify.
									// Pop all due tasks
									let due_tasks = scheduler.pop_due(time::OffsetDateTime::now_utc());

									run_task_sender.send(due_tasks).unwrap();
								}
								None => {
									tracing::info!("waiting, no enabled source to schedule");

									notify_new_enabled_source.notified().await;
								}
							}

							Ok::<(), Error>(())
						} => { result? }
					}

				}
				#[expect(unused, reason = "for type inference")]
				Ok::<(), Error>(())
			} => { result? }
		}

		Ok(())
	}

	/// Create a channel to receive real-time events from the scheduler.
	///
	/// Should be closed using [`super::Representative::close_events_channel()`].
	pub fn open_events_channel(&mut self) -> tokio::sync::mpsc::UnboundedReceiver<Event> {
		let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<Event>();

		self.events_sender = Some(sender);

		receiver
	}

	/// Close the sender of the events channel.
	///
	/// This should be used first instead of closing the receiver to prevent unnecessary attempts by the scheduler to send.
	pub fn close_events_channel(&mut self) {
		self.events_sender = None;
	}

	fn send_event<F>(&self, event: F)
	where
		F: FnOnce() -> Event,
	{
		if let Some(events_sender) = &self.events_sender {
			// NOTE: To avoid executing code when the channel is not open.
			let event = event();

			tracing::debug!(?event, "sending to events channel");

			if let Err(error) = events_sender.send(event) {
				tracing::warn!(
					event=?error.0,
					"Failed to send event due to the receiver being closed. Please make sure to close the sender as well."
				);
			}
		}
	}
}
