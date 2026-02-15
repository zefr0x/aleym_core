#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum NetworkError {
	#[cfg(all(feature = "net_interface_clear", feature = "net_transport_tcp"))]
	#[error("Input/Output error occurred: {0}")]
	IoError(#[from] std::io::Error),
	#[cfg(feature = "net_interface_tor")]
	#[error("Arti error occurred: {0}")]
	ArtiError(#[from] arti_client::Error),
	#[cfg(feature = "net_transport_tls")]
	#[error("TLS error occurred: {0}")]
	RustlsError(#[from] tokio_rustls::rustls::Error),
	#[cfg(feature = "net_transport_tls")]
	#[error("Invalid DNS name error: {0}")]
	InvalidDnsName(#[from] tokio_rustls::rustls::pki_types::InvalidDnsNameError),
}
