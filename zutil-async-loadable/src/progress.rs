//! Progress updater

// Imports
use {crate::Inner, std::sync::Arc};

/// Progress updater
// TODO: Remove `T` type parameter by somehow storing a `MappedArc<Mutex<Option<P>>>`?
pub struct ProgressUpdater<T, P> {
	/// Inner
	inner: Arc<Inner<T, P>>,
}

impl<T, P> ProgressUpdater<T, P> {
	/// Creates a new progress updater
	pub(crate) fn new(inner: Arc<Inner<T, P>>) -> Self {
		Self { inner }
	}

	/// Updates the progress
	pub fn update(&self, progress: P) {
		*self.inner.progress.lock() = Some(progress);
	}

	/// Updates the progress
	pub fn update_with<F>(&self, f: F)
	where
		F: FnOnce(&mut P),
		P: Default,
	{
		// Note: This can't deadlock, as `AsyncLoadable::progress` only
		//       tries to lock, and if it can't, it returns `None`.
		f(self.inner.progress.lock().get_or_insert_default());
	}
}
