use tokio::io::{AsyncRead, AsyncWrite};

use crate::net::NetworkError;
#[cfg(feature = "net_transport_tls")]
use crate::net::transports::tls::AlpnProtocols;

#[cfg(feature = "net_protocol_http2")]
#[tracing::instrument(level = tracing::Level::DEBUG)]
async fn send_http2<T, B>(
	mut request: hyper::Request<T>,
	io: hyper_util::rt::TokioIo<B>,
) -> Result<hyper::Response<hyper::body::Incoming>, NetworkError>
where
	T: hyper::body::Body + Unpin + Send + std::fmt::Debug + 'static,
	T::Data: Send,
	T::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
	B: Unpin + Send + AsyncWrite + AsyncRead + std::fmt::Debug + 'static,
{
	use tracing::Instrument;

	*request.version_mut() = hyper::http::Version::HTTP_2;

	// TODO: Create an HTTP/2 pool, so we maintain the same connection for multiple requests.

	let (mut request_sender, connection) =
		hyper::client::conn::http2::handshake(hyper_util::rt::TokioExecutor::default(), io).await?;

	tokio::spawn(
		async move {
			if let Err(error) = connection.await {
				tracing::error!(?error);
			}
		}
		.instrument(tracing::debug_span!(parent: tracing::Span::current(), "Connection")),
	);

	Ok(request_sender.send_request(request).await?)
}

#[cfg(feature = "net_protocol_http1")]
#[tracing::instrument(level = tracing::Level::DEBUG)]
async fn send_http1<T, B>(
	mut request: hyper::Request<T>,
	io: hyper_util::rt::TokioIo<B>,
) -> Result<hyper::Response<hyper::body::Incoming>, NetworkError>
where
	T: hyper::body::Body + Unpin + Send + std::fmt::Debug + 'static,
	T::Data: Send,
	T::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
	B: Unpin + Send + AsyncWrite + AsyncRead + std::fmt::Debug + 'static,
{
	use tracing::Instrument;

	*request.version_mut() = hyper::http::Version::HTTP_11;

	let (mut request_sender, connection) = hyper::client::conn::http1::handshake(io).await?;

	tokio::spawn(
		async move {
			if let Err(error) = connection.await {
				tracing::error!(?error);
			}
		}
		.instrument(tracing::debug_span!(parent: tracing::Span::current(), "Connection")),
	);

	Ok(request_sender.send_request(request).await?)
}

#[cfg(all(
	any(feature = "net_protocol_http1", feature = "net_protocol_http2"),
	feature = "net_transport_tls"
))]
pub trait Https: super::super::transports::tls::Transport {
	async fn send_request<T>(
		&self,
		request: hyper::Request<T>,
		host: String,
		port: u16,
	) -> Result<hyper::Response<hyper::body::Incoming>, NetworkError>
	where
		T: hyper::body::Body + Send + Unpin + std::fmt::Debug + 'static,
		T::Data: Send,
		T::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
	{
		let (tls_stream, alpn_protocol) = super::super::transports::tls::Transport::connect(self, host, port).await?;
		let io = hyper_util::rt::TokioIo::new(tls_stream);

		match alpn_protocol {
			#[cfg(all(feature = "net_protocol_http2", feature = "net_protocol_http1"))]
			AlpnProtocols::H2 => send_http2(request, io).await,
			#[cfg(feature = "net_protocol_http1")]
			AlpnProtocols::Fallback => send_http1(request, io).await,
			#[cfg(not(feature = "net_protocol_http1"))]
			AlpnProtocols::Fallback => unreachable!(),
		}
	}
}

#[cfg(any(feature = "net_protocol_http1", feature = "net_protocol_http2"))]
pub trait Http: super::super::transports::tcp::Transport {
	async fn send_request<T>(
		&self,
		request: hyper::Request<T>,
		host: String,
		port: u16,
		#[cfg(all(feature = "net_protocol_http2", feature = "net_protocol_http1"))] http2_prior_knowledge: bool,
	) -> Result<hyper::Response<hyper::body::Incoming>, NetworkError>
	where
		T: hyper::body::Body + Send + Unpin + std::fmt::Debug + 'static,
		T::Data: Send,
		T::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
	{
		let tcp_stream = super::super::transports::tcp::Transport::connect(self, &host, port).await?;
		let io = hyper_util::rt::TokioIo::new(tcp_stream);

		// NOTE: RFC 9113 considers h2c upgrade obsolete, so `prior_knowledge` is our only option.
		#[cfg(all(feature = "net_protocol_http2", feature = "net_protocol_http1"))]
		if http2_prior_knowledge {
			return send_http2(request, io).await;
		}

		cfg_if::cfg_if! {
			if #[cfg(all(not(feature = "net_protocol_http2"), feature = "net_protocol_http1"))] {
				return send_http2(request, io).await;
			} else if #[cfg(all(feature = "net_protocol_http1"))] {
				return send_http1(request, io).await;
			}
		}
	}
}

// TODO: Consider using `tower_http` or other higher level library.

// TODO: Handle gzip, brotli, zstd, and deflate decompression.

// TODO: handle HTTP redirects.

// TODO: Support HTTP proxies.

// TODO: Add HTTP/3 support.

// TODO: Clean HttpAuto boilerplate when cfg-attributes in where clauses are supported:
// https://github.com/rust-lang/rfcs/pull/3399

#[cfg(all(
	feature = "net_transport_tls",
	any(feature = "net_protocol_http1", feature = "net_protocol_http2")
))]
pub trait HttpAuto: Http + Https {
	async fn send_request<T>(
		&self,
		request: hyper::Request<T>,
		#[cfg(all(feature = "net_protocol_http2", feature = "net_protocol_http1"))] http2_prior_knowledge: bool,
	) -> Result<hyper::Response<hyper::body::Incoming>, NetworkError>
	where
		T: hyper::body::Body + Send + Unpin + std::fmt::Debug + 'static,
		T::Data: Send,
		T::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
	{
		let uri = request.uri();

		let scheme = uri.scheme_str().ok_or(NetworkError::NoProtocolScheme)?;
		let host = uri.host().ok_or(NetworkError::NoTargetHost)?.to_owned();

		match scheme {
			#[cfg(all(
				feature = "net_transport_tls",
				any(feature = "net_protocol_http1", feature = "net_protocol_http2")
			))]
			"https" => {
				let port = uri.port_u16().unwrap_or(443);

				Https::send_request(self, request, host, port).await
			}
			#[cfg(any(feature = "net_protocol_http1", feature = "net_protocol_http2"))]
			"http" => {
				use super::super::protocols::http::Http;

				let port = uri.port_u16().unwrap_or(80);

				Http::send_request(
					self,
					request,
					host,
					port,
					#[cfg(all(feature = "net_protocol_http2", feature = "net_protocol_http1"))]
					http2_prior_knowledge,
				)
				.await
			}
			_ => Err(NetworkError::UnsupportedProtocolScheme),
		}
	}
}

#[cfg(all(
	feature = "net_transport_tls",
	any(feature = "net_protocol_http1", feature = "net_protocol_http2")
))]
impl<T: Http + Https> HttpAuto for T {}

#[cfg(all(
	not(feature = "net_transport_tls"),
	any(feature = "net_protocol_http1", feature = "net_protocol_http2")
))]
pub trait HttpAuto: Http {
	async fn send_request<T>(
		&self,
		request: hyper::Request<T>,
		#[cfg(all(feature = "net_protocol_http2", feature = "net_protocol_http1"))] http2_prior_knowledge: bool,
	) -> Result<hyper::Response<hyper::body::Incoming>, NetworkError>
	where
		T: hyper::body::Body + Send + Unpin + std::fmt::Debug + 'static,
		T::Data: Send,
		T::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
	{
		let uri = request.uri();

		let scheme = uri.scheme_str().ok_or(NetworkError::NoProtocolScheme)?;
		let host = uri.host().ok_or(NetworkError::NoTargetHost)?.to_owned();

		match scheme {
			#[cfg(any(feature = "net_protocol_http1", feature = "net_protocol_http2"))]
			"http" => {
				use super::super::protocols::http::Http;

				let port = uri.port_u16().unwrap_or(80);

				Http::send_request(
					self,
					request,
					host,
					port,
					#[cfg(all(feature = "net_protocol_http2", feature = "net_protocol_http1"))]
					http2_prior_knowledge,
				)
				.await
			}
			_ => Err(NetworkError::UnsupportedProtocolScheme),
		}
	}
}

#[cfg(all(
	not(feature = "net_transport_tls"),
	any(feature = "net_protocol_http1", feature = "net_protocol_http2")
))]
impl<T: Http> HttpAuto for T {}
