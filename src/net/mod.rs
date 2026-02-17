mod client;
mod error;
mod interfaces;
mod protocols;
mod transports;

#[cfg(feature = "net_transport_tls")]
use std::sync::Arc;

pub use client::Client;
pub use error::NetworkError;
pub use interfaces::Type as InterfaceType;

/// Networking abstraction layer, handling multiple network transports.
///
/// This must be the only interface to all network communications.
pub struct Network {
	#[cfg(feature = "net_interface_tor")]
	tor_client: arti_client::TorClient<tor_rtcompat::PreferredRuntime>,
	#[cfg(feature = "net_transport_tls")]
	tls_config: Arc<tokio_rustls::rustls::ClientConfig>,
}

impl Network {
	/// Initialize connections to different network transports.
	pub async fn new() -> Result<Self, NetworkError> {
		tracing::trace!("initializing network connections");

		#[cfg(feature = "net_transport_tls")]
		let tls_config = {
			use rustls_platform_verifier::BuilderVerifierExt;

			tracing::trace!("configuring TLS");

			// TODO: Expose some of rustls config.
			let mut config = tokio_rustls::rustls::ClientConfig::builder_with_provider(Arc::new(
				tokio_rustls::rustls::crypto::ring::default_provider(),
			))
			.with_safe_default_protocol_versions()?
			.with_platform_verifier()?
			.with_no_client_auth();

			// Supported upper layer protocols
			config.alpn_protocols.extend(vec![
				#[cfg(feature = "net_protocol_http2")]
				b"h2".to_vec(),
				#[cfg(feature = "net_protocol_http1")]
				b"http/1.1".to_vec(),
			]);

			Arc::new(config)
		};

		#[cfg(feature = "net_interface_tor")]
		let tor_client = {
			tracing::trace!("bootstrapping Tor client");

			// TODO: Expose Tor client config.
			let config = arti_client::TorClientConfig::default();

			arti_client::TorClient::builder()
				.config(config)
				.bootstrap_behavior(arti_client::BootstrapBehavior::OnDemand)
				.create_unbootstrapped_async()
				.await?
		};

		Ok(Self {
			#[cfg(feature = "net_transport_tls")]
			tls_config,
			#[cfg(feature = "net_interface_tor")]
			tor_client,
		})
	}

	/// Separate client should be created for each informant execution.
	#[expect(unused)]
	pub async fn new_client(&self, interface: InterfaceType) -> Client {
		// TODO: Expose client specific config overwrites.
		match interface {
			#[cfg(feature = "net_interface_clear")]
			InterfaceType::Clear => Client::Clear(
				interfaces::ClearInterface::new(
					#[cfg(feature = "net_transport_tls")]
					Arc::clone(&self.tls_config),
				)
				.await,
			),
			#[cfg(feature = "net_interface_tor")]
			InterfaceType::Tor => Client::Tor(Box::new(
				interfaces::TorInterface::new(
					self.tor_client.isolated_client(),
					#[cfg(feature = "net_transport_tls")]
					Arc::clone(&self.tls_config),
				)
				.await,
			)),
		}
	}
}

#[cfg(test)]
mod tests {
	#[cfg(all(
		feature = "net_interface_clear",
		feature = "net_transport_tls",
		feature = "net_protocol_http1"
	))]
	#[tokio::test]
	#[tracing_test::traced_test]
	async fn networking_clear_http1_tls() {
		use super::{InterfaceType, Network};
		use http_body_util::BodyExt;

		let network = Network::new().await.unwrap();

		let client = network.new_client(InterfaceType::Clear).await;

		let target = hyper::Uri::from_static("https://check.torproject.org/api/ip");
		let request = hyper::Request::builder()
			.header(hyper::header::HOST, target.authority().unwrap().as_str())
			.uri(&target)
			.method(hyper::Method::GET)
			.body(http_body_util::Empty::<hyper::body::Bytes>::new())
			.unwrap();

		let resp = client.http_request(request, false).await.unwrap();

		assert_eq!(resp.status().as_u16(), 200);
		assert_eq!(resp.version(), hyper::Version::HTTP_11);
		assert_eq!(
			resp.headers().get("content-type"),
			Some(&hyper::header::HeaderValue::from_static("application/json"))
		);

		let body_bytes = resp.collect().await.unwrap().to_bytes();
		let body_str = std::str::from_utf8(&body_bytes).unwrap();

		assert!(body_str.contains("\"IsTor\":false"));
	}

	#[cfg(all(
		feature = "net_interface_tor",
		feature = "net_transport_tls",
		feature = "net_protocol_http1"
	))]
	#[tokio::test]
	#[tracing_test::traced_test]
	async fn networking_tor_http1_tls() {
		use super::{InterfaceType, Network};
		use http_body_util::BodyExt;

		let network = Network::new().await.unwrap();

		let client = network.new_client(InterfaceType::Tor).await;

		let target = hyper::Uri::from_static("https://check.torproject.org/api/ip");
		let request = hyper::Request::builder()
			.header(hyper::header::HOST, target.authority().unwrap().as_str())
			.uri(&target)
			.method(hyper::Method::GET)
			.body(http_body_util::Empty::<hyper::body::Bytes>::new())
			.unwrap();

		let resp = client.http_request(request, false).await.unwrap();

		assert_eq!(resp.status().as_u16(), 200);
		assert_eq!(resp.version(), hyper::Version::HTTP_11);
		assert_eq!(
			resp.headers().get("content-type"),
			Some(&hyper::header::HeaderValue::from_static("application/json"))
		);

		let body_bytes = resp.collect().await.unwrap().to_bytes();
		let body_str = std::str::from_utf8(&body_bytes).unwrap();

		assert!(body_str.contains("\"IsTor\":true"));
	}
}
