#[derive(Debug)]
#[repr(i8)]
pub(crate) enum InformantErrorKind {
	#[cfg(feature = "informant_feedrs")]
	Internal = 0,
	#[cfg(feature = "informant_feedrs")]
	Network = 1,
	#[cfg(feature = "informant_feedrs")]
	Parsing = 2,
	Parameters = 3,
}

impl From<InformantErrorKind> for i8 {
	fn from(value: InformantErrorKind) -> Self {
		value as i8
	}
}

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

impl InformantError {
	/// Get the error kind for database storage.
	///
	/// Used to categorize errors for the machine-learned ranking engine.
	#[expect(unused)]
	pub(crate) const fn kind(&self) -> InformantErrorKind {
		// NOTE: Breaking changes to the following mappings should be accompanied with a database migration.
		match self {
			#[cfg(feature = "informant_feedrs")]
			InformantError::TokioOneShotError(_) => InformantErrorKind::Internal,

			#[cfg(feature = "informant_feedrs")]
			InformantError::NetworkError(_) => InformantErrorKind::Network,

			#[cfg(feature = "informant_feedrs")]
			InformantError::FeedRsParsingError(_) => InformantErrorKind::Parsing,

			InformantError::UnsupportedInformatIdentifier(_) | InformantError::InvalidInformantParameters(_) => {
				InformantErrorKind::Parameters
			}
			#[cfg(feature = "informant_feedrs")]
			InformantError::InvalidUri(_) | InformantError::NoTargetUriAuthority => InformantErrorKind::Parameters,
		}
	}
}
