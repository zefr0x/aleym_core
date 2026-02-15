#[cfg(feature = "net_interface_clear")]
mod clear;

#[cfg(feature = "net_interface_clear")]
pub(super) use clear::ClearInterface;

pub enum Type {
	#[cfg(feature = "net_interface_clear")]
	#[expect(unused)]
	Clear,
}
