//! Pre-initialization logger

// Imports
use {
	std::{
		io,
		sync::{
			Arc,
			nonpoison::{Mutex, MutexGuard},
		},
	},
	tracing::{dispatcher, subscriber::DefaultGuard},
	tracing_subscriber::{EnvFilter, Layer, fmt::MakeWriter, prelude::__tracing_subscriber_SubscriberExt},
};

/// Pre-init logger
pub struct PreInitLogger {
	/// Output
	output: PreInitOutput,

	/// Guard
	_guard: DefaultGuard,
}

impl PreInitLogger {
	/// Creates a new pre-init logger
	pub fn new() -> Self {
		let output = PreInitOutput::default();
		let layer = tracing_subscriber::fmt::layer()
			.with_target(false)
			.with_writer(output.clone())
			.with_ansi(false)
			.with_filter(EnvFilter::from_default_env());

		// Initialize a barebones logger first to catch all logs
		// until our temporary subscriber is up and running.
		let logger = tracing_subscriber::registry().with(layer);
		let guard = dispatcher::set_default(&logger.into());

		Self { output, _guard: guard }
	}

	/// Drops this logger and returns it's output
	pub fn into_output(self) -> PreInitOutput {
		self.output
	}
}

/// Pre-init output
#[derive(Clone, Default, Debug)]
pub struct PreInitOutput(Arc<Mutex<Vec<u8>>>);

impl PreInitOutput {
	/// Uses the bytes in this output
	pub fn with_bytes<O>(&self, f: impl FnOnce(&[u8]) -> O) -> O {
		f(&self.0.lock())
	}
}

impl<'a> MakeWriter<'a> for PreInitOutput {
	type Writer = PreOutputWrite<'a>;

	fn make_writer(&'a self) -> Self::Writer {
		PreOutputWrite(self.0.lock())
	}
}

/// Pre-init output writer
pub struct PreOutputWrite<'a>(MutexGuard<'a, Vec<u8>>);

impl io::Write for PreOutputWrite<'_> {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		self.0.write(buf)
	}

	fn flush(&mut self) -> io::Result<()> {
		self.0.flush()
	}
}
