#[cfg(feature = "net_interface_clear")]
mod clear;
#[cfg(feature = "net_interface_tor")]
mod tor;

#[cfg(feature = "net_interface_clear")]
pub(super) use clear::ClearInterface;
#[cfg(feature = "net_interface_tor")]
pub(super) use tor::TorInterface;

pub enum Type {
	#[cfg(feature = "net_interface_clear")]
	#[expect(unused)]
	Clear,
	#[cfg(feature = "net_interface_tor")]
	#[expect(unused)]
	Tor,
}
