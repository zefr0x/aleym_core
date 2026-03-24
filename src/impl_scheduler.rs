use super::{
	Error,
	db::{AddingNewsOutput, InputNews, uuid::Uuid},
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

	/// Manually execute fetch operation of a specific source.
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
					let items = self
						.trigger_informant(
							super::net::InterfaceType::try_from(source.network)?,
							serde_json::from_value::<inform::Parameters>(source.informant_parameters)
								.map_err(inform::InformantError::from)?,
						)
						.await;

					match items {
						Ok(items) => {
							let output = self.storage.add_news(source.id, items).await?;

							self.send_event(move || Event::NewsUpdated {
								source_id: source.id,
								updates: output,
							});
						}
						Err(e) => {
							self.send_event(move || Event::InformantError {
								source_id: source.id,
								error: e.to_string(),
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
	pub fn open_events_channel(&mut self) -> tokio::sync::mpsc::UnboundedReceiver<Event> {
		let (sender, reciver) = tokio::sync::mpsc::unbounded_channel::<Event>();

		self.events_sender = Some(sender);

		reciver
	}

	fn send_event<F>(&self, event: F)
	where
		F: FnOnce() -> Event,
	{
		if let Some(events_sender) = &self.events_sender {
			// NOTE: To avoid executing code when the channel is not open.
			let event = event();

			tracing::debug!(?event, "sending to events channel");

			events_sender.send(event).unwrap()
		}
	}
}
