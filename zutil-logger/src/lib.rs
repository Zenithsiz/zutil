//! Logger

// Features
#![feature(nonpoison_mutex, sync_nonpoison, anonymous_lifetime_in_impl_trait)]

// Modules
mod file;
mod pre_init;
mod term;

// Imports
use {
	self::{file::FileWriter, pre_init::PreInitLogger},
	itertools::Itertools,
	std::{
		self,
		collections::{HashMap, hash_map},
		env::{self, VarError},
		fs,
		io::Write,
		path::Path,
	},
	tracing::Subscriber,
	tracing_subscriber::{Layer, Registry, fmt::MakeWriter, layer::Layered, prelude::*, registry::LookupSpan},
};

/// Logger
pub struct Logger {
	/// File writer
	file_writer: FileWriter,
}

impl Logger {
	/// Creates a new logger
	///
	/// Starts already logging to stderr.
	pub fn new<W, L>(
		stderr: W,
		extra_layers: L,
		default_stderr_filters: impl IntoIterator<Item = (Option<&'_ str>, &'_ str)>,
		default_file_filters: impl IntoIterator<Item = (Option<&'_ str>, &'_ str)>,
	) -> Self
	where
		W: for<'a> MakeWriter<'a> + Clone + Send + Sync + 'static,
		L: ExtraLayers<LoggerSubscriber>,
		L::Subscriber: Subscriber + for<'a> LookupSpan<'a> + Send + Sync + 'static,
	{
		// Create the pre-init logger to log everything until we have our loggers running.
		let pre_init_logger = PreInitLogger::new();

		// Then initialize our logging
		let file_writer = FileWriter::memory();

		// Note: Due to [this issue](https://github.com/tokio-rs/tracing/issues/1817),
		//       the order here matters, and the stderr ones must be last.
		let subscriber = LoggerSubscriber::default();
		let subscriber = extra_layers
			.layer_on(subscriber)
			.with(file::layer(file_writer.clone(), default_file_filters))
			.with(term::layer(stderr.clone(), default_stderr_filters));
		if let Err(err) = subscriber.try_init() {
			eprintln!("Failed to set global logger: {err}");
		}

		// Finally write the pre-init output to our writes
		pre_init_logger
			.into_output()
			.with_bytes(|bytes| {
				stderr.make_writer().write_all(bytes)?;
				file_writer.make_writer().write_all(bytes)
			})
			.expect("Unable to write pre-init output");

		tracing::info!("Successfully initialized logger");

		Self { file_writer }
	}

	/// Sets a file to log into.
	///
	/// Once the logger is finished, any logs produced until then
	/// will be retro-actively written into this log file.
	pub fn set_file(&self, path: Option<&Path>) {
		match path {
			Some(path) => match fs::File::create(path) {
				Ok(file) => {
					self.file_writer.set_file(file);
					tracing::info!("Logging to file: {path:?}");
				},
				Err(err) => {
					tracing::warn!("Unable to create log file {path:?}: {err}");
					self.file_writer.set_empty()
				},
			},
			None => self.file_writer.set_empty(),
		}
	}
}

/// Logger subscriber
// TODO: Hide this behind a trait impl type alias
pub type LoggerSubscriber = Registry;

/// Returns the env filters of a variable.
///
/// Adds default filters, if not specified
#[must_use]
fn get_env_filters(env: &str, default_filters: impl IntoIterator<Item = (Option<&'_ str>, &'_ str)>) -> String {
	// Get the current filters
	let env_var;
	let mut cur_filters = match env::var(env) {
		// Split filters by `,`, then src and level by `=`
		Ok(var) => {
			env_var = var;
			env_var
				.split(',')
				.map(|s| match s.split_once('=') {
					Some((src, level)) => (Some(src), level),
					None => (None, s),
				})
				.collect::<HashMap<_, _>>()
		},

		// If there were none, don't use any
		Err(err) => {
			if let VarError::NotUnicode(var) = err {
				tracing::warn!("Ignoring non-utf8 env variable {env:?}: {var:?}");
			}

			HashMap::new()
		},
	};

	// Add all default filters, if not specified
	for (src, level) in default_filters {
		if let hash_map::Entry::Vacant(entry) = cur_filters.entry(src) {
			let _ = entry.insert(level);
		}
	}

	// Then re-create it
	let var = cur_filters
		.into_iter()
		.map(|(src, level)| match src {
			Some(src) => format!("{src}={level}"),
			None => level.to_owned(),
		})
		.join(",");
	tracing::trace!("Using {env}={var}");

	var
}

/// Extra layers
pub trait ExtraLayers<S> {
	/// Subscriber with all layers
	type Subscriber;

	/// Layers all layers onto a subscriber
	fn layer_on(self, subscriber: S) -> Self::Subscriber;
}

impl<S> ExtraLayers<S> for () {
	type Subscriber = S;

	fn layer_on(self, subscriber: S) -> Self::Subscriber {
		subscriber
	}
}

impl<S, L> ExtraLayers<S> for (L,)
where
	S: Subscriber,
	L: Layer<S>,
{
	type Subscriber = Layered<L, S>;

	fn layer_on(self, subscriber: S) -> Self::Subscriber {
		subscriber.with(self.0)
	}
}

impl<S, L0, L1> ExtraLayers<S> for (L0, L1)
where
	S: Subscriber,
	L0: Layer<S>,
	L1: Layer<Layered<L0, S>>,
{
	type Subscriber = Layered<L1, Layered<L0, S>>;

	fn layer_on(self, subscriber: S) -> Self::Subscriber {
		subscriber.with(self.0).with(self.1)
	}
}
