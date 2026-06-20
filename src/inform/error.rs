#[derive(Debug)]
#[repr(i8)]
pub(crate) enum InformantErrorKind {
	#[cfg(any(feature = "informant_feedrs", feature = "informant_telegram_web"))]
	Internal = 0,
	#[cfg(any(feature = "informant_feedrs", feature = "informant_telegram_web"))]
	Network = 1,
	#[cfg(any(feature = "informant_feedrs", feature = "informant_telegram_web"))]
	Parsing = 2,
	Parameters = 3,
	#[cfg(feature = "informant_feedrs")]
	InvalidIntraction = 4,
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

	#[cfg(any(feature = "informant_feedrs", feature = "informant_telegram_web"))]
	#[error("Tokio one-shot channel receive error occurred: {0}")]
	TokioOneShotError(#[from] tokio::sync::oneshot::error::RecvError),

	#[cfg(any(feature = "informant_feedrs", feature = "informant_telegram_web"))]
	#[error("Network error occurred: {0}")]
	NetworkError(#[from] crate::net::NetworkError),

	#[cfg(feature = "informant_feedrs")]
	#[error("Reached the HTTP redirection count limit")]
	HttpRedirectionCountLimitReached,
	#[cfg(feature = "informant_feedrs")]
	#[error("HTTP redirection location header doesn't exist")]
	NoHttpRedirectionLocation,
	#[cfg(feature = "informant_feedrs")]
	#[error("HTTP redirection location is invalid: {0}")]
	InvalidHttpRedirectionLocation(url::Url),
	#[cfg(feature = "informant_feedrs")]
	#[error("HTTP redirection with unallowed scheme change from `{from}` to `{to}`")]
	UnallowedRedirectionSchemeChange { from: String, to: String },

	#[cfg(feature = "informant_feedrs")]
	#[error("Invalid header value to string slice access: {0}")]
	InvalidHttpHeaderValueToStr(#[from] crate::net::protocols::http::header::ToStrError),
	#[cfg(any(feature = "informant_feedrs", feature = "informant_telegram_web"))]
	#[error("URL parsing error occurred: {0}")]
	InvalidUrl(#[from] url::ParseError),

	#[cfg(feature = "informant_telegram_web")]
	#[error("Invalid UTF-8 string parsing error occurred: {0}")]
	InvalidUtf8Str(#[from] std::str::Utf8Error),
	#[cfg(feature = "informant_telegram_web")]
	#[error("Invalid escaped string error occurred: {0}")]
	UnescapeError(#[from] unescaper::Error),
	#[cfg(feature = "informant_telegram_web")]
	#[error("Datetime parsing error occurred: {0}")]
	DateTimeParsingError(#[from] time::error::Parse),

	#[cfg(feature = "informant_feedrs")]
	#[error("`feed_rs` parsing error occurred: {0}")]
	FeedRsParsingError(#[from] feed_rs::parser::ParseFeedError),
	#[cfg(feature = "informant_telegram_web")]
	#[error("Undefied format while scraping telegram web")]
	TelegramWebUndefiedFormat,
}

impl InformantError {
	/// Get the error kind for database storage.
	///
	/// Used to categorize errors for the machine-learned ranking engine.
	#[cfg_attr(not(feature = "_informant"), expect(unused))]
	pub(crate) const fn kind(&self) -> InformantErrorKind {
		// NOTE: Breaking changes to the following mappings should be accompanied with a database migration.
		match self {
			#[cfg(any(feature = "informant_feedrs", feature = "informant_telegram_web"))]
			InformantError::TokioOneShotError(_) => InformantErrorKind::Internal,

			#[cfg(any(feature = "informant_feedrs", feature = "informant_telegram_web"))]
			InformantError::NetworkError(_) => InformantErrorKind::Network,

			#[cfg(feature = "informant_feedrs")]
			InformantError::HttpRedirectionCountLimitReached
			| InformantError::NoHttpRedirectionLocation
			| InformantError::InvalidHttpHeaderValueToStr(_)
			| InformantError::InvalidHttpRedirectionLocation(_)
			| InformantError::UnallowedRedirectionSchemeChange { from: _, to: _ } => InformantErrorKind::InvalidIntraction,

			#[cfg(feature = "informant_feedrs")]
			InformantError::FeedRsParsingError(_) => InformantErrorKind::Parsing,
			#[cfg(feature = "informant_telegram_web")]
			InformantError::TelegramWebUndefiedFormat
			| InformantError::UnescapeError(_)
			| InformantError::InvalidUtf8Str(_)
			| InformantError::DateTimeParsingError(_) => InformantErrorKind::Parsing,

			InformantError::UnsupportedInformatIdentifier(_) | InformantError::InvalidInformantParameters(_) => {
				InformantErrorKind::Parameters
			}
			#[cfg(any(feature = "informant_feedrs", feature = "informant_telegram_web"))]
			InformantError::InvalidUrl(_) => InformantErrorKind::Parameters,
		}
	}
}
