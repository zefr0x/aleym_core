use std::pin::Pin;

use super::AsyncStream;
use crate::net::NetworkError;

#[expect(unused)]
pub trait Transport {
	async fn connect(&self, host: &str, port: u16) -> Result<Pin<Box<dyn AsyncStream>>, NetworkError>;
}
