use std::pin::Pin;
use std::sync::Arc;

use super::{AsyncStream, tcp};
use crate::net::NetworkError;

pub enum AlpnProtocols {
	Fallback,
}

#[expect(unused)]
pub trait Transport: super::tcp::Transport {
	fn tls_config(&self) -> Arc<tokio_rustls::rustls::ClientConfig>;

	async fn connect(
		&self,
		host: String,
		port: u16,
	) -> Result<(tokio_rustls::TlsStream<Pin<Box<dyn AsyncStream>>>, AlpnProtocols), NetworkError> {
		let tcp_stream = tcp::Transport::connect(self, &host, port).await?;

		let tls_stream = tokio_rustls::TlsConnector::from(Arc::clone(&self.tls_config()))
			.connect(tokio_rustls::rustls::pki_types::ServerName::try_from(host)?, tcp_stream)
			.await?;

		let alpn_protocol = {
			let (_, tls_client_connection) = tls_stream.get_ref();

			#[expect(clippy::match_single_binding)]
			match tls_client_connection.alpn_protocol() {
				_ => AlpnProtocols::Fallback,
			}
		};

		Ok((tokio_rustls::TlsStream::Client(tls_stream), alpn_protocol))
	}
}
