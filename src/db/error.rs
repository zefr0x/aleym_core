#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum StorageError {
	#[error("Database error occurred: {0}")]
	DatabaseError(#[from] sea_orm::DbErr),
	#[error("Input/Output error occurred: {0}")]
	IoError(#[from] std::io::Error),
	#[error("Path contains non-UTF-8 characters")]
	InvalidUtf8Path,
}
