#[cfg(any(feature = "net_protocol_http1", feature = "net_protocol_http2"))]
mod impl_http;

#[non_exhaustive]
pub enum Client {
	#[cfg(feature = "net_interface_clear")]
	Clear(super::interfaces::ClearInterface),
	// TODO: Analyze if we need Box or not.
	#[cfg(feature = "net_interface_tor")]
	Tor(Box<super::interfaces::TorInterface>),
}
