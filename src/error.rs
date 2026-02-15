use crate::{db, net};

#[expect(missing_docs, reason = "Variants' names and error messages are descriptive")]
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
	#[error("Storage error occurred: {0}")]
	StorageError(#[from] db::StorageError),
	#[error("Network error occurred: {0}")]
	NetworkError(#[from] net::NetworkError),
}
