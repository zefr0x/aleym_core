#[cfg(feature = "net_transport_tcp")]
pub(super) mod tcp;
#[cfg(feature = "net_transport_tls")]
pub(super) mod tls;

#[cfg(feature = "net_transport_tcp")]
pub(super) trait AsyncStream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send {}
