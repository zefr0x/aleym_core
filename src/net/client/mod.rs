#[non_exhaustive]
pub enum Client {
	#[cfg(feature = "net_interface_clear")]
	Clear(super::interfaces::ClearInterface),
}
