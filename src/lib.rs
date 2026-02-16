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
mod net;

pub use db::Migrator as DbMigrator;
pub use error::Error;

pub struct Representative {
	storage: db::Connection,
	#[expect(unused)]
	network: net::Network,
}

impl Representative {
	pub async fn new(database_file: Option<&std::path::Path>) -> Result<Self, Error> {
		tracing::trace!("initializing new Aleym Representative");

		Ok(Self {
			storage: db::Connection::new(database_file).await?,
			network: net::Network::new().await?,
		})
	}

	/// Return `true` if we have any pending migrations.
	///
	/// Useful to prepare the user interface before starting to apply them.
	pub async fn has_pending_migrations(&self) -> Result<bool, Error> {
		Ok(self.storage.has_pending_migrations().await?)
	}

	/// Apply all pending migrations. If there is none, it will silently fail.
	///
	/// This should be executed once after every update to avoid errors.
	pub async fn apply_migrations(&self) -> Result<(), Error> {
		Ok(self.storage.apply_migrations().await?)
	}
}
