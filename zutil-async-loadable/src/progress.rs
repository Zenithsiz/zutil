//! Progress updater

// Imports
use {crate::Inner, mappable_rc::Marc, parking_lot::Mutex, std::sync::Arc};

/// Progress updater
pub struct ProgressUpdater<P: 'static> {
	/// Progress
	progress: Marc<Mutex<Option<P>>>,
}

impl<P> ProgressUpdater<P> {
	/// Creates a new progress updater
	pub(crate) fn new<T>(inner: Arc<Inner<T, P>>) -> Self
	where
		T: Send + 'static,
		P: Send,
	{
		let inner = Marc::from_arc(inner);
		let progress = Marc::map(inner, |inner| &inner.progress);
		Self { progress }
	}

	/// Updates the progress
	pub fn update(&self, progress: P) {
		*self.progress.lock() = Some(progress);
	}

	/// Updates the progress
	pub fn update_with<F>(&self, f: F)
	where
		F: FnOnce(&mut P),
		P: Default,
	{
		// Note: This can't deadlock, as `AsyncLoadable::progress` only
		//       tries to lock, and if it can't, it returns `None`.
		f(self.progress.lock().get_or_insert_default());
	}
}
