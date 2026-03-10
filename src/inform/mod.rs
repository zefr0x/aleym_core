#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(i8)]
pub enum Type {
	// TODO: Restrict this only for tests with `#[cfg(test)]` when there are other variants
	TestPlaceholder = 0,
}

impl From<Type> for sea_orm::Value {
	fn from(value: Type) -> Self {
		sea_orm::Value::TinyInt(Some(value as i8))
	}
}
