//! App errors
//!
//! This crate provides an error type, [`AppError`], that is ideal for usage in apps.
//!
//! It is [`Send`], [`Sync`], `'static`, and importantly cheaply [`Clone`]-able.
//!
//! The inner representation is currently just `Arc<(String, Option<AppError>)>`.

// Features
#![feature(error_reporter, decl_macro)]

// Imports
use std::{
	error::Error as StdError,
	fmt,
	hash::{Hash, Hasher},
	sync::Arc,
};

/// Inner
struct Inner {
	/// Message
	msg: String,

	/// Source
	source: Option<AppError>,
}

impl StdError for Inner {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		self.source.as_ref().map(|err| &err.inner as _)
	}
}

impl fmt::Display for Inner {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.msg.fmt(f)
	}
}

impl fmt::Debug for Inner {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match f.alternate() {
			// With `:#?`, use a normal debug
			true => f
				.debug_struct("AppError")
				.field("msg", &self.msg)
				.field("source", &self.source)
				.finish(),

			// Otherwise, pretty print it
			false => std::error::Report::new(self).pretty(true).fmt(f),
		}
	}
}

/// A reference-counted untyped error that can be created from any error type.
///
/// Named `AppError` as it's mostly useful in apps that don't care about the errors
/// specifically, and instead only care to show them to users.
#[derive(Clone)]
pub struct AppError {
	/// Inner
	inner: Arc<Inner>,
}

impl AppError {
	/// Creates a new app error from an error
	pub fn new<E>(err: &E) -> Self
	where
		E: ?Sized + StdError,
	{
		Self {
			inner: Arc::new(Inner {
				msg:    err.to_string(),
				source: err.source().map(Self::new),
			}),
		}
	}

	/// Creates a new app error from a message
	pub fn msg<M>(msg: M) -> Self
	where
		M: fmt::Display,
	{
		Self {
			inner: Arc::new(Inner {
				msg:    msg.to_string(),
				source: None,
			}),
		}
	}

	/// Adds context to this error
	pub fn context<M>(&self, msg: M) -> Self
	where
		M: fmt::Display,
	{
		Self {
			inner: Arc::new(Inner {
				msg:    msg.to_string(),
				source: Some(self.clone()),
			}),
		}
	}

	/// Returns this type as a [`std::error::Error`]
	pub fn as_std_error(&self) -> &(dyn StdError + 'static) {
		&self.inner
	}

	/// Converts this type as into a [`std::error::Error`]
	pub fn into_std_error(self) -> Arc<dyn StdError + Send + Sync + 'static> {
		self.inner as Arc<_>
	}
}

impl<E> From<E> for AppError
where
	E: StdError,
{
	fn from(err: E) -> Self {
		Self::new(&err)
	}
}

impl PartialEq for AppError {
	fn eq(&self, other: &Self) -> bool {
		// If we're the same Arc, we're the same error
		if Arc::ptr_eq(&self.inner, &other.inner) {
			return true;
		}

		// Otherwise, perform a deep comparison
		self.inner.msg == other.inner.msg && self.inner.source == other.inner.source
	}
}

impl Eq for AppError {}

impl Hash for AppError {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.inner.msg.hash(state);
		self.inner.source.hash(state);
	}
}

impl fmt::Display for AppError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.inner.fmt(f)
	}
}

impl fmt::Debug for AppError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.inner.fmt(f)
	}
}

/// Context for `Result`-like types
pub trait Context {
	type Output;

	/// Adds context to this result, if it's an error
	fn context<M>(self, msg: M) -> Self::Output
	where
		M: fmt::Display;

	/// Adds context to this result lazily, if it's an error
	fn with_context<F, M>(self, with_msg: F) -> Self::Output
	where
		F: FnOnce() -> M,
		M: fmt::Display;
}

impl<T, E> Context for Result<T, E>
where
	E: StdError,
{
	type Output = Result<T, AppError>;

	fn context<M>(self, msg: M) -> Self::Output
	where
		M: fmt::Display,
	{
		self.map_err(|err| AppError::new(&err).context(msg))
	}

	fn with_context<F, M>(self, with_msg: F) -> Self::Output
	where
		F: FnOnce() -> M,
		M: fmt::Display,
	{
		self.map_err(|err| AppError::new(&err).context(with_msg()))
	}
}

impl<T> Context for Result<T, AppError> {
	type Output = Result<T, AppError>;

	fn context<M>(self, msg: M) -> Self::Output
	where
		M: fmt::Display,
	{
		self.map_err(|err| err.context(msg))
	}

	fn with_context<F, M>(self, with_msg: F) -> Self::Output
	where
		F: FnOnce() -> M,
		M: fmt::Display,
	{
		self.map_err(|err| err.context(with_msg()))
	}
}

impl<T> Context for Option<T> {
	type Output = Result<T, AppError>;

	fn context<M>(self, msg: M) -> Self::Output
	where
		M: fmt::Display,
	{
		self.ok_or_else(|| AppError::msg(msg))
	}

	fn with_context<F, M>(self, with_msg: F) -> Self::Output
	where
		F: FnOnce() -> M,
		M: fmt::Display,
	{
		self.ok_or_else(|| AppError::msg(with_msg()))
	}
}

/// A macro that formats and creates an [`AppError`]
pub macro app_error {
	($msg:literal $(,)?) => {
		$crate::AppError::msg( format!($msg) )
	},

	($fmt:literal, $($arg:expr),* $(,)?) => {
		$crate::AppError::msg( format!($fmt, $($arg,)*) )
	},
}
