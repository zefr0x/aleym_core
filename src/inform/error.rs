#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum InformantError {
	#[error("Informat with identifier `{0}` is not available")]
	UnsupportedInformatIdentifier(i8),
}
