//! File logging

// Imports
use {
	std::{
		fs,
		io::{self, Write},
		sync::{
			Arc,
			nonpoison::{Mutex, MutexGuard},
		},
	},
	tracing::Subscriber,
	tracing_subscriber::{EnvFilter, Layer, fmt::MakeWriter, registry::LookupSpan},
};

/// File layer writer.
#[derive(Clone, Debug)]
pub struct FileWriter {
	kind: Arc<Mutex<FileWriterKind>>,
}

impl FileWriter {
	/// Creates a new file writer, writing to memory
	pub fn memory() -> Self {
		Self {
			kind: Arc::new(Mutex::new(FileWriterKind::Memory(vec![]))),
		}
	}

	/// Sets this file writer to write into a file.
	///
	/// If this was writing into memory, writes all captured
	/// data into the file
	pub fn set_file(&self, mut file: fs::File) {
		let mut kind = self.kind.lock();
		if let FileWriterKind::Memory(bytes) = &*kind &&
			let Err(err) = file.write_all(bytes)
		{
			tracing::warn!("Unable to write to log file: {err}")
		}

		*kind = FileWriterKind::File(file);
	}

	/// Sets this file writer to become empty
	pub fn set_empty(&self) {
		*self.kind.lock() = FileWriterKind::None;
	}
}

impl<'a> MakeWriter<'a> for FileWriter {
	type Writer = FileWriterKindGuard<'a>;

	fn make_writer(&'a self) -> Self::Writer {
		FileWriterKindGuard(self.kind.lock())
	}
}

/// Backend for the file writer.
#[derive(Debug)]
enum FileWriterKind {
	/// File
	File(fs::File),

	/// Memory
	Memory(Vec<u8>),

	/// None
	None,
}

/// Guard that implements `io::Write` for `FileWriter` to return
#[derive(Debug)]
pub struct FileWriterKindGuard<'a>(MutexGuard<'a, FileWriterKind>);

impl io::Write for FileWriterKindGuard<'_> {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		match &mut *self.0 {
			FileWriterKind::File(file) => file.write(buf),
			FileWriterKind::Memory(items) => items.write(buf),
			FileWriterKind::None => Ok(0),
		}
	}

	fn flush(&mut self) -> io::Result<()> {
		match &mut *self.0 {
			FileWriterKind::File(file) => file.flush(),
			FileWriterKind::Memory(items) => items.flush(),
			FileWriterKind::None => Ok(()),
		}
	}
}

/// Creates the file layer
pub fn layer<S>(
	writer: FileWriter,
	default_filters: impl IntoIterator<Item = (Option<&'_ str>, &'_ str)>,
) -> impl Layer<S>
where
	S: Subscriber + for<'a> LookupSpan<'a>,
{
	// Then create the layer
	let env = super::get_env_filters("RUST_FILE_LOG", default_filters);
	let layer = tracing_subscriber::fmt::layer()
		.with_writer(writer)
		.with_ansi(false)
		.with_filter(EnvFilter::builder().parse_lossy(env));

	Some(layer)
}
