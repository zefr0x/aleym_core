#[cfg(feature = "net_transport_tcp")]
use crate::net::transports::{AsyncStream, tcp};

pub struct ClearInterface {}

#[cfg(feature = "net_transport_tcp")]
impl AsyncStream for tokio::net::TcpStream {}

#[cfg(feature = "net_transport_tcp")]
impl tcp::Transport for ClearInterface {
	async fn connect(
		&self,
		host: &str,
		port: u16,
	) -> Result<std::pin::Pin<Box<dyn AsyncStream>>, crate::net::NetworkError> {
		Ok(Box::pin(tokio::net::TcpStream::connect((host, port)).await?))
	}
}

impl ClearInterface {
	pub async fn new() -> Self {
		Self {}
	}
}
