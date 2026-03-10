#[cfg(feature = "net_interface_clear")]
mod clear;
#[cfg(feature = "net_interface_tor")]
mod tor;

#[cfg(feature = "net_interface_clear")]
pub(super) use clear::ClearInterface;
#[cfg(feature = "net_interface_tor")]
pub(super) use tor::TorInterface;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(i8)]
pub enum Type {
	#[cfg(any(test, not(any(feature = "net_interface_clear", feature = "net_interface_tor"))))]
	TestPlaceholder = 0,
	#[cfg(feature = "net_interface_clear")]
	Clear = 1,
	#[cfg(feature = "net_interface_tor")]
	Tor = 2,
}

impl From<Type> for sea_orm::Value {
	fn from(value: Type) -> Self {
		sea_orm::Value::TinyInt(Some(value as i8))
	}
}
