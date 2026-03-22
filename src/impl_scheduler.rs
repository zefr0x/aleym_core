#[cfg(feature = "_informant")]
use super::{
	Error,
	db::{AddingNewsOutput, InputNews, uuid::Uuid},
	inform::{self, InformantTrait as _},
};

impl super::Representative {
	/// Manually execute specific informant using a specific network.
	#[cfg(feature = "_informant")]
	async fn trigger_informant(
		&self,
		network: i8,
		informant: i8,
		informant_parameters: sea_orm::JsonValue,
	) -> Result<Vec<InputNews>, Error> {
		#[cfg(feature = "informant_feedrs")]
		let network = self.network.new_client(super::net::InterfaceType::try_from(network)?);

		match inform::Type::try_from(informant)? {
			#[cfg(any(test, not(feature = "_informant")))]
			inform::Type::TestPlaceholder => {
				unimplemented!()
			}
			#[cfg(feature = "informant_feedrs")]
			inform::Type::FeedRs => Ok(inform::feedrs::Informant::new(network)
				.execute(informant_parameters)
				.await?),
		}
	}

	/// Manually execute fetch operation of a specific source.
	#[cfg(feature = "_informant")]
	pub async fn trigger_informant_by_source(&self, source: Uuid) -> Result<AddingNewsOutput, Error> {
		let source = self.storage.get_source(source).await?;

		let items = self
			.trigger_informant(source.network, source.informant, source.informant_parameters)
			.await?;

		Ok(self.storage.add_news(source.id, items).await?)
	}
}
