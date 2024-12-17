//! Arc guard to the result

// Imports
use {
	super::{Inner, Res},
	mappable_rc::Marc,
	parking_lot::{Mutex, MutexGuard},
	stable_deref_trait::StableDeref,
	std::{ops::Deref, sync::Arc},
	yoke::Yoke,
};

/// Result mapped arc.
struct ResMarc<T: 'static>(pub Marc<Mutex<Res<T>>>);

impl<T> ResMarc<T> {
	/// Creates a mapped arc to the result from an arc to inner.
	pub fn new<P>(inner: Arc<Inner<T, P>>) -> Self
	where
		T: Send,
		P: Send + 'static,
	{
		let inner = Marc::from_arc(inner);
		let inner_res = Marc::map(inner, |inner| &inner.res);
		Self(inner_res)
	}
}

impl<T> Deref for ResMarc<T> {
	type Target = Mutex<Res<T>>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

// SAFETY: We hold an `Arc`, `Deref` always returns the same pointer,
//         and do not implement `DerefMut`.
unsafe impl<T> StableDeref for ResMarc<T> {}

/// Result guard
#[derive(yoke::Yokeable)]
struct ResGuard<'a, T>(pub MutexGuard<'a, Res<T>>);

/// Arc guard to the result
pub struct ResArcGuard<T: 'static>(Yoke<ResGuard<'static, T>, ResMarc<T>>);

impl<T> ResArcGuard<T> {
	/// Creates an arc guard to the result.
	pub fn new<P>(inner: Arc<Inner<T, P>>) -> ResArcGuard<T>
	where
		T: Send,
		P: Send + 'static,
	{
		let inner = Yoke::attach_to_cart(ResMarc::new(inner), |inner| ResGuard(inner.lock()));
		Self(inner)
	}

	/// Gets the inner result
	pub fn get(&self) -> &Res<T> {
		&self.0.get().0
	}

	/// Modifies the inner result
	pub fn with_mut<F>(&mut self, f: F)
	where
		F: FnOnce(&mut Res<T>) + 'static,
	{
		self.0.with_mut(|inner| f(&mut inner.0));
	}
}
