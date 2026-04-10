use super::{
	Error,
	db::{AddingNewsOutput, InputNews, SourceFeedbackSignal, StorageError, uuid::Uuid},
	inform::{self, InformantError, InformantTrait as _},
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
		#[cfg(feature = "informant_feedrs")]
		let network = self.network.new_client(network);

		match informant_parameters {
			#[cfg(any(test, not(feature = "_informant")))]
			inform::Parameters::TestPlaceholder => {
				unimplemented!()
			}
			#[cfg(feature = "informant_feedrs")]
			inform::Parameters::FeedRs(parameters) => Ok(inform::feedrs::Informant::new(network).execute(parameters).await?),
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

	/// This never returns unless storage related error occurs.
	pub async fn start_scheduler(&self) -> Result<(), Error> {
		use rand::seq::SliceRandom as _;

		let mut rng = rand::rng();

		loop {
			let mut sources = self.storage.get_all_sources(Some(true)).await?;
			sources.shuffle(&mut rng);

			loop {
				tokio::time::sleep(std::time::Duration::from_mins(rand::random_range(1..15))).await;

				if let Some(source) = sources.pop() {
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
									new_items_count: addition_output
										.new
										.len()
										.try_into()
										.map_err(StorageError::from)?,
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
				} else {
					break;
				}
			}
		}
	}

	/// Create a channel to receive real-time events from the scheduler.
	///
	/// Should be closed using [`close_events_channel()`].
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
