#[cfg(feature = "net_transport_tls")]
use std::sync::Arc;

#[cfg(feature = "net_transport_tls")]
use crate::net::transports::tls;
#[cfg(feature = "net_transport_tcp")]
use crate::net::transports::{AsyncStream, tcp};

pub struct ClearInterface {
	#[cfg(feature = "net_transport_tls")]
	tls_config: Arc<tokio_rustls::rustls::ClientConfig>,
}

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

#[cfg(feature = "net_transport_tls")]
impl tls::Transport for ClearInterface {
	fn tls_config(&self) -> Arc<tokio_rustls::rustls::ClientConfig> {
		Arc::clone(&self.tls_config)
	}
}

impl ClearInterface {
	pub async fn new(#[cfg(feature = "net_transport_tls")] tls: Arc<tokio_rustls::rustls::ClientConfig>) -> Self {
		Self {
			#[cfg(feature = "net_transport_tls")]
			tls_config: tls,
		}
	}
}
