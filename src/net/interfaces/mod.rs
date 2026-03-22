#[cfg(feature = "net_interface_clear")]
mod clear;
#[cfg(feature = "net_interface_tor")]
mod tor;

#[cfg(feature = "net_interface_clear")]
pub(super) use clear::ClearInterface;
#[cfg(feature = "net_interface_tor")]
pub(super) use tor::TorInterface;

use crate::net::NetworkError;

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

impl TryFrom<i8> for Type {
	type Error = NetworkError;

	fn try_from(value: i8) -> Result<Self, Self::Error> {
		match value {
			#[cfg(any(test, not(any(feature = "net_interface_clear", feature = "net_interface_tor"))))]
			0 => Ok(Self::TestPlaceholder),
			#[cfg(feature = "net_interface_clear")]
			1 => Ok(Self::Clear),
			#[cfg(feature = "net_interface_tor")]
			2 => Ok(Self::Tor),
			value => Err(NetworkError::UnsupportedNetworkInterfaceIdentifier(value)),
		}
	}
}

impl From<Type> for sea_orm::Value {
	fn from(value: Type) -> Self {
		sea_orm::Value::TinyInt(Some(value as i8))
	}
}
