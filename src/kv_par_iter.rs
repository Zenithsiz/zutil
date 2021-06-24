//! Key-value parallel iterator

// Imports
use either::Either;

/// Iterator over two key-value pair iterators, providing either both values for
/// an equal key, or just either side otherwise.
pub struct KVParIter<L, R>
where
	L: Iterator<Item: KeyValue>,
	R: Iterator<Item = L::Item>,
{
	/// Left iterator
	left: L,

	/// Right iterator
	right: R,

	/// Currently cached value
	// Option<L::Key, Either<L::Value, R::Value>>
	#[allow(clippy::type_complexity)] // We can't easily simplify it
	value: Option<(
		<L::Item as KeyValue>::Key,
		Either<<L::Item as KeyValue>::Value, <R::Item as KeyValue>::Value>,
	)>,
}

impl<L, R> KVParIter<L, R>
where
	L: Iterator,
	L::Item: KeyValue,
	R: Iterator<Item = L::Item>,
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
		if let Some((key, value)) = self.value.take() {
			match value {
				// If we did, return it
				Either::Left(value) => return Some((key, value).into()),
				// If it wasn't on the left, put it back
				Either::Right(_) => self.value = Some((key, value)),
			}
		}

		// If we didn't have it cached get it from the iterator
		self.left.next()
	}

	/// Returns the next right value
	pub fn next_right(&mut self) -> Option<R::Item> {
		// Check if we have it cached
		if let Some((key, value)) = self.value.take() {
			match value {
				// If we did, return it
				Either::Right(value) => return Some((key, value).into()),
				// If it wasn't on the right, put it back
				Either::Left(_) => self.value = Some((key, value)),
			}
		}

		// If we didn't have it cached get it from the iterator
		self.right.next()
	}
}

impl<L, R> Iterator for KVParIter<L, R>
where
	L: Iterator,
	L::Item: KeyValue,
	<L::Item as KeyValue>::Key: Ord,
	R: Iterator<Item = L::Item>,
{
	// Option<L::Key, ParIterValue<L::Value, R::Value>>
	#[allow(clippy::type_complexity)] // We can't easily simplify it
	type Item = (
		<L::Item as KeyValue>::Key,
		ParIterValue<<L::Item as KeyValue>::Value, <R::Item as KeyValue>::Value>,
	);

	fn next(&mut self) -> Option<Self::Item> {
		match (self.next_left().map(Into::into), self.next_right().map(Into::into)) {
			// If we only got one of each value, just return it
			(Some((key, left)), None) => Some((key, ParIterValue::Left(left))),
			(None, Some((key, right))) => Some((key, ParIterValue::Right(right))),

			// If we got both with equal keys, return them both
			(Some((left_key, left)), Some((right_key, right))) if left_key == right_key => {
				Some((left_key, ParIterValue::Both(left, right)))
			},

			// If we got both, but without equal keys, emit the first and store the other.
			// Note: In all of these branches, `self.value` is empty, as we call both `self.next_{left, right}`
			//       functions.
			(Some((left_key, left)), Some((right_key, right))) => match left_key < right_key {
				true => {
					self.value = Some((right_key, Either::Right(right)));
					Some((left_key, ParIterValue::Left(left)))
				},
				false => {
					self.value = Some((left_key, Either::Left(left)));
					Some((right_key, ParIterValue::Right(right)))
				},
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
}


/// Key-Value pair
pub trait KeyValue: Into<(Self::Key, Self::Value)> + From<(Self::Key, Self::Value)> {
	/// Key
	type Key;

	/// Value
	type Value;
}

impl<K, V> KeyValue for (K, V) {
	type Key = K;
	type Value = V;
}
