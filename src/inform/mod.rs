mod error;

pub use error::InformantError;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(i8)]
pub(crate) enum Type {
	// TODO: Restrict this only for tests with `#[cfg(test)]` when there are other variants
	TestPlaceholder = 0,
}

/// # Informant Parameters
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub enum Parameters {
	/// # Placeholder for Testing
	// TODO: Restrict this only for tests with `#[cfg(test)]` when there are other variants
	TestPlaceholder,
}

impl TryFrom<i8> for Type {
	type Error = InformantError;

	fn try_from(value: i8) -> Result<Self, Self::Error> {
		match value {
			0 => Ok(Self::TestPlaceholder),
			value => Err(InformantError::UnsupportedInformatIdentifier(value)),
		}
	}
}

impl From<&Parameters> for Type {
	fn from(value: &Parameters) -> Self {
		match value {
			Parameters::TestPlaceholder => Type::TestPlaceholder,
		}
	}
}

#[expect(unused)]
pub(crate) trait InformantTrait {
	/// New informant interface should be created for each fetching operation.
	fn new(network_client: crate::net::Client) -> Self;

	async fn execute(self, parameters: sea_orm::JsonValue) -> Result<Vec<crate::db::InputNews>, InformantError>;
}
