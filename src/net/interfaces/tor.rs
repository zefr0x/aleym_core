#[cfg(feature = "net_transport_tcp")]
use crate::net::transports::{AsyncStream, tcp};

pub struct TorInterface {
	#[expect(unused)]
	client: arti_client::TorClient<tor_rtcompat::PreferredRuntime>,
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

impl TorInterface {
	pub async fn new(tor_client: arti_client::TorClient<tor_rtcompat::PreferredRuntime>) -> Self {
		Self { client: tor_client }
	}
}
