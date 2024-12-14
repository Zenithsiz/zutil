//! Key-value parallel iterator

// Imports
use {either::Either, std::cmp::Ordering};

/// Iterator over two keyed sorted iterators, providing both
/// elements when keys are equal.
///
/// Otherwise, keys are returned in order.
pub struct KeyedParIter<L, R>
where
	L: Iterator,
	R: Iterator,
{
	/// Left iterator
	left: L,

	/// Right iterator
	right: R,

	/// Currently cached value
	value: Option<Either<L::Item, R::Item>>,
}

impl<L, R> KeyedParIter<L, R>
where
	L: Iterator,
	R: Iterator,
{
	/// Creates a new iterator
	#[must_use]
	pub fn new(left: impl IntoIterator<IntoIter = L>, right: impl IntoIterator<IntoIter = R>) -> Self {
		Self {
			left:  left.into_iter(),
			right: right.into_iter(),
			value: None,
		}
	}

	/// Returns the next left value
	pub fn next_left(&mut self) -> Option<L::Item> {
		// Check if we have it cached
		match self.value.take() {
			// If we did, return it
			Some(Either::Left(value)) => return Some(value),
			// Else put it back
			value => self.value = value,
		}

		// If we didn't have it cached get it from the iterator
		self.left.next()
	}

	/// Returns the next right value
	pub fn next_right(&mut self) -> Option<R::Item> {
		// Check if we have it cached
		match self.value.take() {
			// If we did, return it
			Some(Either::Right(value)) => return Some(value),
			// Else put it back
			value => self.value = value,
		}

		// If we didn't have it cached get it from the iterator
		self.right.next()
	}
}

impl<L, R> Iterator for KeyedParIter<L, R>
where
	L: Iterator<Item: Keyed>,
	R: Iterator<Item: Keyed>,
	<L::Item as Keyed>::Key: PartialOrd<<R::Item as Keyed>::Key>,
{
	type Item = ParIterValue<L::Item, R::Item>;

	fn next(&mut self) -> Option<Self::Item> {
		match (self.next_left(), self.next_right()) {
			// If we only got one of each value, just return it
			(Some(value), None) => Some(ParIterValue::Left(value)),
			(None, Some(value)) => Some(ParIterValue::Right(value)),

			// If we got both, compare them
			(Some(left), Some(right)) => {
				let ord = PartialOrd::partial_cmp(left.key(), right.key()).expect("An ordering is required");
				let (value, ret) = match ord {
					Ordering::Less => (Some(Either::Right(right)), ParIterValue::Left(left)),
					Ordering::Greater => (Some(Either::Left(left)), ParIterValue::Right(right)),
					Ordering::Equal => (None, ParIterValue::Both(left, right)),
				};
				assert!(self.value.is_none(), "`self.value` should be empty");
				self.value = value;
				Some(ret)
			},

			// Else we got none
			(None, None) => None,
		}
	}
}

/// Iterator value
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum ParIterValue<L, R> {
	/// Only left
	Left(L),

	/// Only right
	Right(R),

	/// Both
	Both(L, R),
}

impl<L, R> ParIterValue<L, R> {
	/// Returns a pair of options describing this value
	#[must_use]
	#[allow(clippy::missing_const_for_fn)] // False positive
	pub fn into_opt_pair(self) -> (Option<L>, Option<R>) {
		match self {
			Self::Left(left) => (Some(left), None),
			Self::Right(right) => (None, Some(right)),
			Self::Both(left, right) => (Some(left), Some(right)),
		}
	}

	/// Returns the key of this value
	///
	/// Note: When this value is [`Self::Both`], returns the left key
	pub fn key(&self) -> &L::Key
	where
		L: Keyed,
		R: Keyed<Key = L::Key>,
	{
		match self {
			Self::Left(value) | Self::Both(value, _) => value.key(),
			Self::Right(value) => value.key(),
		}
	}

	/// Maps both possible values of this value
	pub fn map<L2, R2>(self, lhs: impl FnOnce(L) -> L2, rhs: impl FnOnce(R) -> R2) -> ParIterValue<L2, R2> {
		match self {
			Self::Left(left) => ParIterValue::Left(lhs(left)),
			Self::Right(right) => ParIterValue::Right(rhs(right)),
			Self::Both(left, right) => ParIterValue::Both(lhs(left), rhs(right)),
		}
	}
}

impl<K, L, R> ParIterValue<(K, L), (K, R)> {
	/// Splits the key and value off of this value
	///
	///
	/// Note: When this value is [`Self::Both`], returns the left key
	#[allow(clippy::missing_const_for_fn)] // False positive
	pub fn key_value(self) -> (K, ParIterValue<L, R>) {
		match self {
			ParIterValue::Left((key, left)) => (key, ParIterValue::Left(left)),
			ParIterValue::Right((key, right)) => (key, ParIterValue::Right(right)),
			ParIterValue::Both((key, left), (_, right)) => (key, ParIterValue::Both(left, right)),
		}
	}
}

/// Keyed value
pub trait Keyed {
	/// Key
	type Key;

	/// Returns this value's key
	fn key(&self) -> &Self::Key;
}

// Used for most `*Map` iterators.
impl<K, V> Keyed for (K, V) {
	type Key = K;

	fn key(&self) -> &Self::Key {
		&self.0
	}
}
