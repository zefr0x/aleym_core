#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum StorageError {
	#[error("Database error occurred: {0}")]
	DatabaseError(#[from] sea_orm::DbErr),
	#[error("Database transaction error occurred: {0}")]
	DatabaseTransactionError(#[from] sea_orm::TransactionError<sea_orm::DbErr>),
	#[error("Input/Output error occurred: {0}")]
	IoError(#[from] std::io::Error),
	#[error("Path contains non-UTF-8 characters")]
	InvalidUtf8Path,
	#[error("Supplied JSON value doesn't match the required structure: {0}")]
	InvalidJsonParameters(#[from] serde_json::Error),
}
