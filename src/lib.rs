//! # Aleym (Core Library)
//!
//! Core library powering Aleym knowledge base and news aggregation system.
//!
//! ## Feature flags
//!
//! * `net_interface_clear` (default) -- build with support for clear interface to the internet
//! * `net_interface_tor` -- build with support for Tor interface
//! * `net_transport_tls` -- build with TLS encryption support (e.g. for HTTPS)
//! * `net_protocol_http1` (default) -- HTTP/1.1 support
//! * `net_protocol_http2` (default) -- HTTP/2 support

mod db;
mod error;
mod inform;
mod net;

pub use db::*;
pub use error::Error;
pub use inform::Type as InformantType;
pub use net::InterfaceType as NetworkInterfaceType;

pub struct Representative {
	// TODO: Consider if this should be exposed directly or not.
	pub storage: db::StorageConnection,
	#[expect(unused)]
	network: net::Network,
}

impl Representative {
	pub async fn new(database_file: Option<&std::path::Path>) -> Result<Self, Error> {
		tracing::trace!("initializing new Aleym Representative");

		Ok(Self {
			storage: db::StorageConnection::new(database_file).await?,
			network: net::Network::new().await?,
		})
	}
}
