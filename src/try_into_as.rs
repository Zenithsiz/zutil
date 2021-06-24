//! Try into as

// Imports
use std::convert::TryInto;

/// Helper for [`TryInto`] to use turbofish
pub trait TryIntoAs: Sized {
	/// Tries to convert `Self` into `T` using `TryInto`
	/// with type annotations.
	fn try_into_as<T>(self) -> Result<T, Self::Error>
	where
		Self: TryInto<T>;
}

impl<U> TryIntoAs for U {
	fn try_into_as<T>(self) -> Result<T, <Self as TryInto<T>>::Error>
	where
		Self: TryInto<T>,
	{
		self.try_into()
	}
}
