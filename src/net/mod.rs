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
pub struct Network {}

impl Network {
	/// Initialize connections to different network transports.
	pub async fn new() -> Result<Self, NetworkError> {
		tracing::trace!("initializing network connections");

		Ok(Self {})
	}

	/// Separate client should be created for each informant execution.
	#[expect(unused)]
	pub async fn new_client(&self, interface: InterfaceType) -> Client {
		// TODO: Expose client specific config overwrites.
		match interface {
			#[cfg(feature = "net_interface_clear")]
			InterfaceType::Clear => Client::Clear(interfaces::ClearInterface::new().await),
		}
	}
}
