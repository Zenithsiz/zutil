//! Iterator adaptors

// Imports
use std::iter::FromIterator;


/// Try map ok
pub struct TryMapOk<I, F> {
	/// Iterator
	iter: I,

	/// Function
	f: F,
}

impl<T, E, U, I: Iterator<Item = Result<T, E>>, F: FnMut(T) -> Result<U, E>> Iterator for TryMapOk<I, F> {
	type Item = Result<U, E>;

	fn next(&mut self) -> Option<Self::Item> {
		self.iter.next().map(|value| value.and_then(&mut self.f))
	}
}

/// Extension trait for [`TryMapOk`]
pub trait TryMapOkIter<T, E>: Iterator<Item = Result<T, E>> + Sized {
	/// Creates a [`TryMapOk`] from this iterator
	fn try_map_ok<F, U>(self, f: F) -> TryMapOk<Self, F>
	where
		F: FnMut(T) -> Result<U, E>,
	{
		TryMapOk { iter: self, f }
	}
}

impl<T, E, I: Iterator<Item = Result<T, E>> + Sized> TryMapOkIter<T, E> for I {}


/// Map error
pub struct MapErr<I, F> {
	/// Iterator
	iter: I,

	/// Function
	f: F,
}

impl<T, E, E2, I: Iterator<Item = Result<T, E>>, F: FnMut(E) -> E2> Iterator for MapErr<I, F> {
	type Item = Result<T, E2>;

	fn next(&mut self) -> Option<Self::Item> {
		self.iter.next().map(|value| value.map_err(&mut self.f))
	}
}

/// Extension trait for [`MapErr`]
pub trait MapErrIter<T, E>: Iterator<Item = Result<T, E>> + Sized {
	/// Creates a [`MapErr`] from this iterator
	fn map_err<F, E2>(self, f: F) -> MapErr<Self, F>
	where
		F: FnMut(E) -> E2,
	{
		MapErr { iter: self, f }
	}
}

impl<T, E, I: Iterator<Item = Result<T, E>> + Sized> MapErrIter<T, E> for I {}


/// Iterator length that may be collected.
#[derive(Clone, Copy, Debug)]
pub struct IterLen {
	/// Length of iterator
	len: usize,
}

impl IterLen {
	/// Returns the number of items the iterator had
	#[must_use]
	pub const fn len(&self) -> usize {
		self.len
	}

	/// Returns if the iterator was empty
	#[must_use]
	pub const fn is_empty(&self) -> bool {
		self.len() == 0
	}
}

impl<A> FromIterator<A> for IterLen {
	fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
		Self {
			len: iter.into_iter().count(),
		}
	}
}
