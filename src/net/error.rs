#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum NetworkError {
	#[cfg(all(feature = "net_interface_clear", feature = "net_transport_tcp"))]
	#[error("Input/Output error occurred: {0}")]
	IoError(#[from] std::io::Error),
}
