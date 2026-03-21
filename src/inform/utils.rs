pub fn spawn_cpu_blocking<F, T>(f: F) -> tokio::sync::oneshot::Receiver<T>
where
	F: FnOnce() -> T + Send + 'static,
	T: Send + core::fmt::Debug + 'static,
{
	let (send, recv) = tokio::sync::oneshot::channel();

	let thread_span = tracing::trace_span!(parent: tracing::Span::current(), "RayonThread");

	rayon::spawn(move || {
		thread_span.in_scope(|| {
			send.send(f()).unwrap();
		});
	});

	recv
}
