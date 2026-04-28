//! Logger builder

use {
	crate::{
		Logger,
		LoggerSubscriber,
		file::{self, FileWriter},
		pre_init::PreInitLogger,
		term,
	},
	std::{
		collections::HashMap,
		io::{self, Write},
	},
	tracing::Subscriber,
	tracing_subscriber::{fmt::MakeWriter, prelude::*, registry::LookupSpan},
};

/// Logger builder
pub struct LoggerBuilder<W, S> {
	/// Pre-initialization logger
	pre_init_logger: PreInitLogger,

	/// Stderr
	stderr: W,

	/// Subscriber
	subscriber: S,

	/// Stderr filters
	stderr_filters: HashMap<Option<String>, String>,

	/// Filter filters
	file_filters: HashMap<Option<String>, String>,
}

impl LoggerBuilder<fn() -> io::Stderr, LoggerSubscriber> {
	/// Creates a new builder
	#[must_use]
	pub fn new() -> Self {
		// Create the pre-init logger to log everything until we have our loggers running.
		let pre_init_logger = PreInitLogger::new();

		Self {
			pre_init_logger,
			stderr: io::stderr,
			subscriber: LoggerSubscriber::default(),
			stderr_filters: [(None, "info".to_owned())].into(),
			file_filters: [(None, "debug".to_owned())].into(),
		}
	}
}

impl<W, S> LoggerBuilder<W, S> {
	/// Sets the stderr output of this logger
	pub fn stderr<W2>(self, stderr: W2) -> LoggerBuilder<W2, S> {
		LoggerBuilder { stderr, ..self }
	}

	/// Adds a layer to this logger's subscriber
	pub fn layer<L>(self, layer: L) -> LoggerBuilder<W, tracing_subscriber::layer::Layered<L, S>>
	where
		S: Subscriber,
		L: tracing_subscriber::Layer<S>,
	{
		LoggerBuilder {
			subscriber: self.subscriber.with(layer),
			..self
		}
	}

	/// Sets the default stderr filter
	#[must_use]
	pub fn stderr_filter_default(mut self, filter: &str) -> Self {
		self.stderr_filters.insert(None, filter.to_owned());
		self
	}

	/// Sets a stderr filter
	#[must_use]
	pub fn stderr_filter(mut self, key: &str, filter: &str) -> Self {
		self.stderr_filters.insert(Some(key.to_owned()), filter.to_owned());
		self
	}

	/// Sets the default file filter
	#[must_use]
	pub fn file_filter_default(mut self, filter: &str) -> Self {
		self.file_filters.insert(None, filter.to_owned());
		self
	}

	/// Sets a file filter
	#[must_use]
	pub fn file_filter(mut self, key: &str, filter: &str) -> Self {
		self.file_filters.insert(Some(key.to_owned()), filter.to_owned());
		self
	}

	/// Sets a filter for both the stderr and file layers
	#[must_use]
	pub fn filter(self, key: &str, filter: &str) -> Self {
		self.stderr_filter(key, filter).file_filter(key, filter)
	}

	/// Builds the logger
	#[must_use]
	pub fn build(self) -> Logger
	where
		W: for<'a> MakeWriter<'a> + Clone + Send + Sync + 'static,
		S: Subscriber + for<'a> LookupSpan<'a> + Send + Sync + 'static,
	{
		// Then initialize our logging
		let file_writer = FileWriter::memory();

		// Note: Due to [this issue](https://github.com/tokio-rs/tracing/issues/1817),
		//       the order here matters, and the stderr ones must be last.
		let file_layer = file::layer(file_writer.clone(), self::filters_iter(&self.file_filters));
		let term_layer = term::layer(self.stderr.clone(), self::filters_iter(&self.stderr_filters));
		let subscriber = self.subscriber.with(file_layer).with(term_layer);
		if let Err(err) = subscriber.try_init() {
			eprintln!("Failed to set global logger: {err}");
		}

		// Finally write the pre-init output to our writes
		if let Err(err) = self.pre_init_logger.into_output().with_bytes(|bytes| {
			self.stderr.make_writer().write_all(bytes)?;
			file_writer.make_writer().write_all(bytes)
		}) {
			tracing::warn!("Unable to write pre-init output: {err:?}");
		}

		tracing::info!("Successfully initialized logger");

		Logger { file_writer }
	}
}

impl Default for LoggerBuilder<fn() -> io::Stderr, LoggerSubscriber> {
	fn default() -> Self {
		Self::new()
	}
}

/// Converts the filters field into an iterator
fn filters_iter(filters: &HashMap<Option<String>, String>) -> impl Iterator<Item = (Option<&'_ str>, &'_ str)> {
	filters.iter().map(|(key, value)| (key.as_deref(), value.as_str()))
}
