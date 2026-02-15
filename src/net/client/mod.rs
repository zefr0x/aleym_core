#[non_exhaustive]
pub enum Client {
	#[cfg(feature = "net_interface_clear")]
	Clear(super::interfaces::ClearInterface),
	// TODO: Analyze if we need Box or not.
	#[cfg(feature = "net_interface_tor")]
	Tor(#[expect(unused)] Box<super::interfaces::TorInterface>),
}
