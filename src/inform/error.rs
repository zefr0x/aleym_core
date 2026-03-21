#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum InformantError {
	#[error("Informat with identifier `{0}` is not available")]
	UnsupportedInformatIdentifier(i8),
	#[error("Informant parameters doesn't match the required structure: {0}")]
	InvalidInformantParameters(#[from] serde_json::Error),

	#[cfg(feature = "informant_feedrs")]
	#[error("URI parsing error occurred: {0}")]
	InvalidUri(#[from] crate::net::protocols::http::InvalidUri),
	#[cfg(feature = "informant_feedrs")]
	#[error("URI contains no host")]
	NoTargetUriAuthority,
	#[cfg(feature = "informant_feedrs")]
	#[error("Network error occurred: {0}")]
	NetworkError(#[from] crate::net::NetworkError),
	#[cfg(feature = "informant_feedrs")]
	#[error("Tokio one-shot channel receive error occurred: {0}")]
	TokioOneShotError(#[from] tokio::sync::oneshot::error::RecvError),
	#[cfg(feature = "informant_feedrs")]
	#[error("`feed_rs` parsing error occurred: {0}")]
	FeedRsParsingError(#[from] feed_rs::parser::ParseFeedError),
}
