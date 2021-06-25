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
	format_args_capture,
	seek_stream_len,
	unboxed_closures,
	fn_traits,
	decl_macro,
	array_methods,
	str_internals,
	auto_traits,
	negative_impls,
	try_trait_v2,
	never_type,
	control_flow_enum,
	unwrap_infallible,
	associated_type_bounds
)]
// Lints
#![warn(clippy::restriction, clippy::pedantic, clippy::nursery)]
// We'll disable the ones we don't need
#![allow(clippy::blanket_clippy_restriction_lints)]
// No unsafe allowed in this crate
#![forbid(unsafe_code)]
// Must use `expect` instead of `unwrap`
#![forbid(clippy::unwrap_used)]
// We don't need to mark every public function `inline`
#![allow(clippy::missing_inline_in_public_items)]
// We prefer literals to be copy-paste-able rather than readable
#![allow(clippy::unreadable_literal)]
// We prefer suffixes to be glued to the literal
#![allow(clippy::unseparated_literal_suffix)]
// We're fine with panicking when entering an unexpected state
#![allow(
	clippy::panic,
	clippy::unreachable,
	clippy::expect_used,
	clippy::panic_in_result_fn,
	clippy::unwrap_in_result,
	clippy::indexing_slicing,
	clippy::todo
)]
// We prefer tail calls
#![allow(clippy::implicit_return)]
// We use multiple implementations to separate logic
#![allow(clippy::multiple_inherent_impl)]
// We use granular error types, usually one for each function, which document the
// errors that might happen, as opposed to documenting them in the function
#![allow(clippy::missing_errors_doc)]
// Due to our module organization, we end up with data types inheriting their module's name
#![allow(clippy::module_name_repetitions)]
// We need arithmetic for this crate
#![allow(clippy::integer_arithmetic, clippy::integer_division)]
// We want to benefit from match ergonomics where possible
#![allow(clippy::pattern_type_mismatch)]
// We only use wildcards when we only care about certain variants
#![allow(clippy::wildcard_enum_match_arm, clippy::match_wildcard_for_single_variants)]
// We're fine with shadowing, as long as it's related
#![allow(clippy::shadow_reuse, clippy::shadow_same)]
// Matching on booleans can look better than `if / else`
#![allow(clippy::match_bool, clippy::single_match_else, clippy::if_not_else)]
// If the `else` isn't needed, we don't put it
#![allow(clippy::else_if_without_else)]
// We're fine with non-exhaustive structs / enums, we aren't committing to them yet.
#![allow(clippy::exhaustive_structs, clippy::exhaustive_enums)]
// There are too many false positives with these lints
#![allow(clippy::use_self)]
// `Header` and `Reader` are common names
#![allow(clippy::similar_names)]
// We only use `# Panics` where a panic might be caused by a mis-use of the user, not assertions
#![allow(clippy::missing_panics_doc)]
// Some errors don't carry enough information to include them in another super-error
#![allow(clippy::map_err_ignore)]

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
pub mod kv_par_iter;
pub mod lock_poison;
pub mod map_box;
pub mod next_from_bytes;
pub mod null_ascii_string;
pub mod signed_hex;
pub mod string_contains_case_insensitive;
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
pub use kv_par_iter::KVParIter;
pub use lock_poison::{MutexPoison, RwLockPoison};
pub use map_box::MapBoxResult;
pub use next_from_bytes::NextFromBytes;
pub use null_ascii_string::NullAsciiString;
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
};
#[cfg(feature = "use_serde")]
use std::{fs, io, path::Path};

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

/// Attempts to create a folder. Returns `Ok` if it already exists.
#[allow(clippy::create_dir)] // We only want to create a single level
pub fn try_create_folder(path: impl AsRef<std::path::Path>) -> Result<(), std::io::Error> {
	match std::fs::create_dir(&path) {
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
