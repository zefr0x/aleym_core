use std::pin::Pin;

use super::AsyncStream;
use crate::net::NetworkError;

pub trait Transport {
	async fn connect(&self, host: &str, port: u16) -> Result<Pin<Box<dyn AsyncStream>>, NetworkError>;
}

#[cfg(any(feature = "net_protocol_http1", feature = "net_protocol_http2"))]
impl<T: Transport> crate::net::protocols::http::Http for T {}
