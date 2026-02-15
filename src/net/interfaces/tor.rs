#[cfg(feature = "net_transport_tls")]
use std::sync::Arc;

#[cfg(feature = "net_transport_tls")]
use crate::net::transports::tls;
#[cfg(feature = "net_transport_tcp")]
use crate::net::transports::{AsyncStream, tcp};

pub struct TorInterface {
	client: arti_client::TorClient<tor_rtcompat::PreferredRuntime>,
	#[cfg(feature = "net_transport_tls")]
	tls_config: Arc<tokio_rustls::rustls::ClientConfig>,
}

#[cfg(feature = "net_transport_tcp")]
impl AsyncStream for arti_client::DataStream {}

#[cfg(feature = "net_transport_tcp")]
impl tcp::Transport for TorInterface {
	async fn connect(
		&self,
		host: &str,
		port: u16,
	) -> Result<std::pin::Pin<Box<dyn AsyncStream>>, crate::net::NetworkError> {
		Ok(Box::pin(self.client.connect((host, port)).await?))
	}
}

#[cfg(feature = "net_transport_tls")]
impl tls::Transport for TorInterface {
	fn tls_config(&self) -> Arc<tokio_rustls::rustls::ClientConfig> {
		Arc::clone(&self.tls_config)
	}
}

impl TorInterface {
	pub async fn new(
		tor_client: arti_client::TorClient<tor_rtcompat::PreferredRuntime>,
		#[cfg(feature = "net_transport_tls")] tls: Arc<tokio_rustls::rustls::ClientConfig>,
	) -> Self {
		Self {
			client: tor_client,
			#[cfg(feature = "net_transport_tls")]
			tls_config: tls,
		}
	}
}
