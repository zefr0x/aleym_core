mod client;
mod error;
mod interfaces;
mod transports;

pub use client::Client;
pub use error::NetworkError;
pub use interfaces::Type as InterfaceType;

/// Networking abstraction layer, handling multiple network transports.
///
/// This must be the only interface to all network communications.
pub struct Network {
	#[cfg(feature = "net_interface_tor")]
	tor_client: arti_client::TorClient<tor_rtcompat::PreferredRuntime>,
}

impl Network {
	/// Initialize connections to different network transports.
	pub async fn new() -> Result<Self, NetworkError> {
		tracing::trace!("initializing network connections");

		#[cfg(feature = "net_interface_tor")]
		let tor_client = {
			tracing::trace!("bootstrapping Tor client");

			// TODO: Expose Tor client config.
			let config = arti_client::TorClientConfig::default();

			arti_client::TorClient::builder()
				.config(config)
				.bootstrap_behavior(arti_client::BootstrapBehavior::OnDemand)
				.create_unbootstrapped_async()
				.await?
		};

		Ok(Self {
			#[cfg(feature = "net_interface_tor")]
			tor_client,
		})
	}

	/// Separate client should be created for each informant execution.
	#[expect(unused)]
	pub async fn new_client(&self, interface: InterfaceType) -> Client {
		// TODO: Expose client specific config overwrites.
		match interface {
			#[cfg(feature = "net_interface_clear")]
			InterfaceType::Clear => Client::Clear(interfaces::ClearInterface::new().await),
			#[cfg(feature = "net_interface_tor")]
			InterfaceType::Tor => Client::Tor(Box::new(
				interfaces::TorInterface::new(self.tor_client.isolated_client()).await,
			)),
		}
	}
}
