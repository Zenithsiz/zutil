//! Utilities
//!
//! This crate is composed of random utilities I make whenever I'm generalizing a concept
//! found elsewhere, or just need to share some code between two workspaces.
//!
//! # Documentation
//! Documentation is pretty much nonexistent.
//! If a feature is used enough to require extensive documentation it will likely be
//! moved to another crate. Thus everything in here is unlikely to ever receive documentation,
//! unless moved elsewhere.
//!
//! # Stability
//! The crate is also unlikely to be moved from `0.1.0`, with features added and removed without
//! any version bump.

// Features
#![feature(
	slice_index_methods,
	seek_stream_len,
	unboxed_closures,
	fn_traits,
	decl_macro,
	auto_traits,
	negative_impls,
	try_trait_v2,
	never_type,
	unwrap_infallible,
	tuple_trait
)]

// Modules
#[cfg(feature = "alert")]
pub mod alert;
pub mod alphabet;
pub mod array_split;
pub mod ascii_str_arr;
#[cfg(feature = "gui")]
pub mod ascii_text_buffer;
pub mod bcd;
pub mod btree_map_vector;
pub mod cached_value;
pub mod discarding_sorted_merge_iter;
pub mod display_wrapper;
pub mod family;
pub mod file_lock;
pub mod io_slice;
pub mod iter;
pub mod keyed_par_iter;
pub mod lock_poison;
pub mod map_box;
pub mod next_from_bytes;
pub mod null_ascii_string;
//pub mod ok_or_return;
pub mod signed_hex;
pub mod string_contains_case_insensitive;
#[cfg(feature = "use_futures")]
pub mod task;
pub mod try_into_as;
pub mod try_or_empty;
pub mod void;
pub mod write_take;

// Exports
pub use alphabet::{Alphabet, StrAlphabet, StrArrAlphabet, StringAlphabet};
pub use ascii_str_arr::AsciiStrArr;
#[cfg(feature = "gui")]
pub use ascii_text_buffer::AsciiTextBuffer;
pub use bcd::BcdU8;
pub use btree_map_vector::BTreeMapVector;
pub use cached_value::CachedValue;
pub use discarding_sorted_merge_iter::DiscardingSortedMergeIter;
pub use display_wrapper::DisplayWrapper;
pub use family::{ResultFamily, Tuple2Family};
pub use file_lock::FileLock;
pub use io_slice::IoSlice;
pub use iter::{IterLen, MapErr, TryMapOk};
pub use keyed_par_iter::KeyedParIter;
pub use lock_poison::{MutexPoison, RwLockPoison};
pub use map_box::MapBoxResult;
pub use next_from_bytes::NextFromBytes;
pub use null_ascii_string::NullAsciiString;
//pub use ok_or_return::{OkOrReturn, OkOrReturnResidual, OkOrReturnResult};
pub use signed_hex::SignedHex;
pub use string_contains_case_insensitive::StrContainsCaseInsensitive;
pub use try_into_as::TryIntoAs;
pub use try_or_empty::TryOrEmpty;
pub use void::Void;
pub use write_take::WriteTake;

// Imports
use std::{
	collections::hash_map::DefaultHasher,
	error, fmt,
	hash::{Hash, Hasher},
	io,
};
#[cfg(feature = "use_serde")]
use std::{fs, path::Path};

/// Error for [`parse_from_file`]
#[cfg(feature = "use_serde")]
#[derive(Debug, thiserror::Error)]
pub enum ParseFromFileError<E: fmt::Debug + error::Error + 'static> {
	/// Unable to open file
	#[error("Unable to open file")]
	Open(#[source] io::Error),

	/// Unable to parse the file
	#[error("Unable to parse file")]
	Parse(#[source] E),
}

/// Opens and parses a value from a file
#[cfg(feature = "use_serde")]
pub fn parse_from_file<
	'de,
	T: serde::Deserialize<'de>,
	E: fmt::Debug + error::Error + 'static,
	P: ?Sized + AsRef<Path>,
>(
	path: &P, parser: fn(fs::File) -> Result<T, E>,
) -> Result<T, ParseFromFileError<E>> {
	let file = fs::File::open(path).map_err(ParseFromFileError::Open)?;
	parser(file).map_err(ParseFromFileError::Parse)
}

/// Error for [`write_to_file`]
#[cfg(feature = "use_serde")]
#[derive(Debug, thiserror::Error)]
pub enum WriteToFileError<E: fmt::Debug + error::Error + 'static> {
	/// Unable to create file
	#[error("Unable to create file")]
	Create(#[source] io::Error),

	/// Unable to write the file
	#[error("Unable to write file")]
	Write(#[source] E),
}

/// Creates and writes a value to a file
#[cfg(feature = "use_serde")]
pub fn write_to_file<T: serde::Serialize, E: fmt::Debug + error::Error + 'static, P: ?Sized + AsRef<Path>>(
	path: &P, value: &T, writer: fn(fs::File, &T) -> Result<(), E>,
) -> Result<(), WriteToFileError<E>> {
	let file = fs::File::create(path).map_err(WriteToFileError::Create)?;
	writer(file, value).map_err(WriteToFileError::Write)
}

/// Returns the absolute different between `a` and `b`, `a - b` as a `i64`.
///
/// # Panics
/// If the result would not fit into a `i64`, a panic occurs.
#[allow(clippy::as_conversions)] // We check every operation
#[allow(clippy::panic)] // Rust panics on failed arithmetic operations by default
#[must_use]
pub fn abs_diff(a: u64, b: u64) -> i64 {
	let diff = if a > b { a - b } else { b - a };

	if diff > i64::MAX as u64 {
		panic!("Overflow when computing signed distance between `u64`");
	}

	#[allow(clippy::cast_possible_wrap)] // We've verified, `diff` is less than `i64::MAX`
	if a > b {
		diff as i64
	} else {
		-(diff as i64)
	}
}

/// Adds a `i64` to a `u64`, performing `a + b`.
///
/// If smaller than `0`, returns 0, if larger than `u64::MAX`, return `u64::MAX`
#[allow(clippy::as_conversions)] // We check every operation
#[allow(clippy::cast_sign_loss)] // We've verify it's positive / negative
#[must_use]
pub const fn saturating_signed_offset(a: u64, b: i64) -> u64 {
	// If `b` is positive, check for overflows. Else check for underflows
	if b > 0 {
		a.saturating_add(b as u64)
	} else {
		let neg_b = match b.checked_neg() {
			Some(neg_b) => neg_b as u64,
			None => i64::MAX as u64 + 1,
		};
		a.saturating_sub(neg_b)
	}
}

/// Adds a `i64` to a `u64`, performing `a + b`.
///
/// If smaller than `0` or larger than `u64::MAX`, returns `None`
#[allow(clippy::as_conversions)] // We check every operation
#[allow(clippy::cast_sign_loss)] // We've verify it's positive / negative
#[must_use]
pub const fn checked_signed_offset(a: u64, b: i64) -> Option<u64> {
	// If `b` is positive, check for overflows. Else check for underflows
	if b > 0 {
		a.checked_add(b as u64)
	} else {
		let neg_b = match b.checked_neg() {
			Some(neg_b) => neg_b as u64,
			None => i64::MAX as u64 + 1,
		};
		a.checked_sub(neg_b)
	}
}

/// Adds a `i64` to a `u64`, performing `a + b`.
///
/// If smaller than `0` or larger than `u64::MAX`, panics
#[allow(clippy::as_conversions)] // We check every operation
#[allow(clippy::cast_sign_loss)] // We've verify it's positive / negative
#[must_use]
pub const fn signed_offset(a: u64, b: i64) -> u64 {
	if b > 0 {
		a + b as u64
	} else {
		let neg_b = match b.checked_neg() {
			Some(neg_b) => neg_b as u64,
			None => i64::MAX as u64 + 1,
		};
		a - neg_b
	}
}

/// Prints an error
pub fn fmt_err(err: &(dyn error::Error + '_), f: &mut fmt::Formatter) -> fmt::Result {
	write!(f, "{err}")?;

	let mut source = err.source();
	for n in 1usize.. {
		match source {
			Some(err) => {
				write!(f, "\n  {n}: {err}")?;
				source = err.source();
			},
			None => break,
		}
	}

	Ok(())
}

/// Returns a wrapper that prints an error
pub fn fmt_err_wrapper<'a>(err: &'a (dyn error::Error + 'a)) -> impl fmt::Display + 'a {
	DisplayWrapper::new(move |f| self::fmt_err(err, f))
}

/// Returns a wrapper that prints an error that owns the error
pub fn fmt_err_wrapper_owned<E: error::Error>(err: E) -> impl fmt::Display {
	DisplayWrapper::new(move |f| self::fmt_err(&err, f))
}

// TODO: Rename both of these `try_*` to like `*_if_{not}_exists`.

/// Attempts to, recursively, create a directory.
///
/// Returns `Ok` if it already exists
pub fn try_create_dir_all(path: impl AsRef<std::path::Path>) -> Result<(), std::io::Error> {
	match std::fs::create_dir_all(&path) {
		Ok(_) => Ok(()),
		// If it already exists, ignore
		Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
		Err(err) => Err(err),
	}
}

/// Attempts to remove a file. Returns `Ok` if it didn't exist.
pub fn try_remove_file(path: impl AsRef<std::path::Path>) -> Result<(), std::io::Error> {
	match std::fs::remove_file(&path) {
		Ok(_) => Ok(()),
		// If it didn't exist, ignore
		Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
		Err(err) => Err(err),
	}
}

/// Calculates the hash of any single value
pub fn hash_of<T: Hash>(value: &T) -> u64 {
	let mut state = DefaultHasher::new();
	value.hash(&mut state);
	state.finish()
}

/// Helper to read an array of bytes from a reader
pub trait ReadByteArray {
	/// Reads a byte array, `[u8; N]` from this reader
	fn read_byte_array<const N: usize>(&mut self) -> Result<[u8; N], std::io::Error>;
}

impl<R: ?Sized + std::io::Read> ReadByteArray for R {
	fn read_byte_array<const N: usize>(&mut self) -> Result<[u8; N], std::io::Error> {
		let mut bytes = [0; N];
		self.read_exact(&mut bytes)?;
		Ok(bytes)
	}
}

/// Helper for [`DisplayWrapper`] to create it out of a formatting string
pub macro display_wrapper( $( $args:tt )* ) {
	$crate::DisplayWrapper::new(|f| {
		write!(f, $( $args )*)
	})
}

/// Reads into a slice until eof.
///
/// Returns the remaining non-filled buffer.
// Note: Based on the `default_read_exact` function in `std`.
pub fn read_slice_until_eof<'a, R: io::Read + ?Sized>(
	reader: &mut R, mut buffer: &'a mut [u8],
) -> Result<&'a mut [u8], ReadSliceUntilEofError> {
	loop {
		match reader.read(buffer) {
			Ok(0) => return Ok(buffer),
			Ok(n) => match buffer.get_mut(n..) {
				Some(new_buf) => buffer = new_buf,
				None => return Err(ReadSliceUntilEofError::FilledBuffer),
			},
			Err(e) if e.kind() == io::ErrorKind::Interrupted => (),
			Err(e) => return Err(ReadSliceUntilEofError::Io(e)),
		}
	}
}

/// Error for [`read_slice_until_eof`]
#[derive(Debug, thiserror::Error)]
pub enum ReadSliceUntilEofError {
	/// Io
	#[error(transparent)]
	Io(io::Error),

	/// Filled the whole buffer before eof.
	#[error("Filled the whole buffer before eof")]
	FilledBuffer,
}

/// Sign extends a `u{N}` to a `u128`
pub fn sign_extend_un(value: u128, n: usize) -> u128 {
	// Shift to left so that msb of `u{N}` is at msb of `u128`.
	let shifted = (value << (128 - n)) as i128;

	// Then shift back, and all bits will be 1 if negative, else 0
	(shifted >> (128 - n)) as u128
}
