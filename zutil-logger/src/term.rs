//! Terminal logging

// Imports
use {
	tracing::{Subscriber, metadata::LevelFilter},
	tracing_subscriber::{EnvFilter, Layer, fmt::MakeWriter, registry::LookupSpan},
};

/// Creates the terminal layer
pub fn layer<W, S>(stderr: W, default_filters: impl IntoIterator<Item = (Option<&'_ str>, &'_ str)>) -> impl Layer<S>
where
	W: for<'a> MakeWriter<'a> + 'static,
	S: Subscriber + for<'a> LookupSpan<'a>,
{
	let env = super::get_env_filters("RUST_LOG", default_filters);
	let layer = tracing_subscriber::fmt::layer().with_target(false).with_writer(stderr);

	#[cfg(debug_assertions)]
	let layer = layer.with_file(true).with_line_number(true).with_thread_names(true);

	layer.with_filter(
		EnvFilter::builder()
			.with_default_directive(LevelFilter::INFO.into())
			.parse_lossy(env),
	)
}
