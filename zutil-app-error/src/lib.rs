//! App errors
//!
//! This crate provides an error type, [`AppError`], that is ideal for usage in apps.
//!
//! It is [`Send`], [`Sync`], `'static`, and importantly cheaply [`Clone`]-able.
//!
//! It is also able to store multiple errors at once and provide pretty-printing of all
//! of these errors.
//!
//! The inner representation is currently just `Arc<(String, Option<AppError>) | Box<[AppError]>>`.

// Features
#![feature(error_reporter, decl_macro, try_trait_v2, extend_one)]

// Modules
mod multiple;
mod pretty;

// Exports
pub use self::{multiple::AllErrs, pretty::PrettyDisplay};

// Imports
use {
	core::mem,
	std::{
		error::Error as StdError,
		fmt,
		hash::{Hash, Hasher},
		sync::Arc,
	},
};

/// Inner
enum Inner {
	/// Single error
	Single {
		/// Message
		msg: String,

		/// Source
		source: Option<AppError>,
	},

	/// Multiple errors
	Multiple(Box<[AppError]>),
}

impl StdError for Inner {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Inner::Single { source, .. } => source.as_ref().map(AppError::as_std_error),
			// For standard errors, just use the first source.
			Inner::Multiple(errs) => errs.first().map(AppError::as_std_error),
		}
	}
}

impl PartialEq for Inner {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(
				Self::Single {
					msg: lhs_msg,
					source: lhs_source,
				},
				Self::Single {
					msg: rhs_msg,
					source: rhs_source,
				},
			) => lhs_msg == rhs_msg && lhs_source == rhs_source,
			(Self::Multiple(lhs), Self::Multiple(rhs)) => lhs == rhs,
			_ => false,
		}
	}
}

impl Hash for Inner {
	fn hash<H: Hasher>(&self, state: &mut H) {
		mem::discriminant(self).hash(state);
		match self {
			Inner::Single { msg, source } => {
				msg.hash(state);
				source.hash(state);
			},
			Inner::Multiple(errs) => errs.hash(state),
		}
	}
}

impl fmt::Display for Inner {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Inner::Single { msg, .. } => msg.fmt(f),
			Inner::Multiple(errs) => write!(f, "Multiple errors ({})", errs.len()),
		}
	}
}

impl fmt::Debug for Inner {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match f.alternate() {
			// With `:#?`, use a normal debug
			true => match self {
				Inner::Single { msg, source } => f
					.debug_struct("AppError")
					.field("msg", msg)
					.field("source", source)
					.finish(),
				Inner::Multiple(errs) => f.debug_list().entries(errs).finish(),
			},

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
			inner: Arc::new(Inner::Single {
				msg:    err.to_string(),
				source: err.source().map(Self::new),
			}),
		}
	}

	/// Creates a new app error from multiple errors
	pub fn from_multiple<Errs>(errs: Errs) -> Self
	where
		Errs: IntoIterator<Item = AppError>,
	{
		Self {
			inner: Arc::new(Inner::Multiple(errs.into_iter().collect())),
		}
	}

	/// Creates a new app error from multiple standard errors
	pub fn from_multiple_std<'a, Errs, E>(errs: Errs) -> Self
	where
		Errs: IntoIterator<Item = &'a E>,
		E: ?Sized + StdError + 'a,
	{
		Self {
			inner: Arc::new(Inner::Multiple(errs.into_iter().map(Self::new).collect())),
		}
	}

	/// Creates a new app error from a message
	pub fn msg<M>(msg: M) -> Self
	where
		M: fmt::Display,
	{
		Self {
			inner: Arc::new(Inner::Single {
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
			inner: Arc::new(Inner::Single {
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

	/// Returns an object that can be used for a pretty display of this error
	#[must_use]
	pub fn pretty(&self) -> PrettyDisplay<'_> {
		PrettyDisplay::new(self)
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
		self.inner == other.inner
	}
}

impl Eq for AppError {}

impl Hash for AppError {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.inner.hash(state);
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
	($msg:expr $(,)?) => {
		$crate::AppError::msg( format!($msg) )
	},

	($fmt:expr, $($arg:expr),* $(,)?) => {
		$crate::AppError::msg( format!($fmt, $($arg,)*) )
	},
}

/// A macro that returns an error
pub macro bail {
	($msg:expr $(,)?) => {
		do yeet $crate::app_error!($msg);
	},

	($fmt:expr, $($arg:expr),* $(,)?) => {
		do yeet $crate::app_error!($fmt, $($arg),*);
	},
}

/// A macro that returns an error if a condition is false
pub macro ensure {
	($cond:expr, $msg:expr $(,)?) => {
		if !$cond {
			do yeet $crate::app_error!($msg);
		}
	},

	($cond:expr, $fmt:expr, $($arg:expr),* $(,)?) => {
		if !$cond {
			do yeet $crate::app_error!($fmt, $($arg),*);
		}
	},
}
