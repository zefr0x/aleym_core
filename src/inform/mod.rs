mod error;
#[cfg(feature = "informant_feedrs")]
pub mod feedrs;
#[cfg(feature = "informant_feedrs")]
mod utils;

pub use error::*;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(i8)]
pub(crate) enum Type {
	#[cfg(any(test, not(feature = "_informant")))]
	TestPlaceholder = 0,
	#[cfg(feature = "informant_feedrs")]
	FeedRs = 1,
}

/// # Informant Parameters
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub enum Parameters {
	/// # Placeholder for Testing
	#[cfg(any(test, not(feature = "_informant")))]
	TestPlaceholder,
	/// # FeedRs
	/// RSS, ATOM, or JSON standard feed.
	#[cfg(feature = "informant_feedrs")]
	FeedRs(
		/// # Parameters
		feedrs::Parameters,
	),
}

impl TryFrom<i8> for Type {
	type Error = InformantError;

	fn try_from(value: i8) -> Result<Self, Self::Error> {
		match value {
			#[cfg(any(test, not(feature = "_informant")))]
			0 => Ok(Self::TestPlaceholder),
			#[cfg(feature = "informant_feedrs")]
			1 => Ok(Self::FeedRs),
			value => Err(InformantError::UnsupportedInformatIdentifier(value)),
		}
	}
}

impl From<&Parameters> for Type {
	fn from(value: &Parameters) -> Self {
		match value {
			#[cfg(any(test, not(feature = "_informant")))]
			Parameters::TestPlaceholder => Type::TestPlaceholder,
			#[cfg(feature = "informant_feedrs")]
			Parameters::FeedRs(_) => Type::FeedRs,
		}
	}
}

#[cfg(feature = "_informant")]
pub(crate) trait InformantTrait {
	type Parameters;

	/// New informant interface should be created for each fetching operation.
	fn new(network_client: crate::net::Client) -> Self;

	async fn execute(self, parameters: Self::Parameters) -> Result<Vec<crate::db::InputNews>, InformantError>;
}
