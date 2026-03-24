//! # Aleym (Core Library)
//!
//! Core library powering Aleym knowledge base and news aggregation system.
//!
//! ## Feature flags
//!
//! * `full` -- build with all features enabled
//! * `net_interface_clear` (default) -- build with support for clear interface to the internet
//! * `net_interface_tor` -- build with support for Tor interface
//! * `net_transport_tls` -- build with TLS encryption support (e.g. for HTTPS)
//! * `net_protocol_http1` (default) -- HTTP/1.1 support
//! * `net_protocol_http2` (default) -- HTTP/2 support
//! * `informant_feedrs` -- build with support for RSS, ATOM, and JSON feeds

pub mod db;
mod error;
#[cfg(feature = "_informant")]
mod impl_scheduler;
pub mod inform;
pub mod net;

pub use error::Error;
#[cfg(feature = "_informant")]
pub use impl_scheduler::Event;

pub struct Representative {
	// TODO: Consider if this should be exposed directly or not.
	pub storage: db::StorageConnection,
	#[allow(unused)]
	network: net::Network,
	#[cfg(feature = "_informant")]
	events_sender: Option<tokio::sync::mpsc::UnboundedSender<Event>>,
}

impl Representative {
	pub async fn new(database_file: Option<&std::path::Path>) -> Result<Self, Error> {
		tracing::trace!("initializing new Aleym Representative");

		Ok(Self {
			storage: db::StorageConnection::new(database_file).await?,
			network: net::Network::new().await?,
			#[cfg(feature = "_informant")]
			events_sender: None,
		})
	}
}
