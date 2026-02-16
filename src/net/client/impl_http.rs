use super::Client;
use crate::net::{NetworkError, protocols::http::HttpAuto};

impl Client {
	/// Note that HTTP Version will be overwritten based on negotiation with the server, setting one is useless.
	///
	/// `http2_prior_knowledge` is useful when we have prior knowledge that the server supports HTTP/2 Cleartext (h2c),
	/// leading to the use of HTTP/2 rather than HTTP/1.1 when we are not doing a TLS handshake for unencrypted connection.
	#[expect(unused)]
	pub async fn http_request<T>(
		&self,
		request: hyper::Request<T>,
		#[cfg(all(feature = "net_protocol_http2", feature = "net_protocol_http1"))] http2_prior_knowledge: bool,
	) -> Result<hyper::Response<hyper::body::Incoming>, NetworkError>
	where
		T: hyper::body::Body + Send + Unpin + std::fmt::Debug + 'static,
		T::Data: Send,
		T::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
	{
		match self {
			#[cfg(feature = "net_interface_clear")]
			Client::Clear(clear) => {
				clear
					.send_request(
						request,
						#[cfg(all(feature = "net_protocol_http2", feature = "net_protocol_http1"))]
						http2_prior_knowledge,
					)
					.await
			}
			#[cfg(feature = "net_interface_tor")]
			Client::Tor(tor) => {
				tor.send_request(
					request,
					#[cfg(all(feature = "net_protocol_http2", feature = "net_protocol_http1"))]
					http2_prior_knowledge,
				)
				.await
			}
			#[cfg(all(not(feature = "net_interface_clear"), not(feature = "net_interface_tor")))]
			_ => compile_error!("At least one network interface must be enabled."),
		}
	}
}
