//! Progress updater

// Imports
use {crate::Inner, parking_lot::Mutex, std::sync::Arc};

/// Progress updater
pub struct ProgressUpdater<T, P> {
	/// Inner
	inner: Arc<Mutex<Inner<T, P>>>,
}

impl<T, P> ProgressUpdater<T, P> {
	/// Creates a new progress updater
	pub(crate) fn new(inner: Arc<Mutex<Inner<T, P>>>) -> Self {
		Self { inner }
	}

	/// Updates the progress
	pub fn update(&self, progress: P) {
		self.inner.lock().progress = Some(progress);
	}

	/// Updates the progress
	// TODO: This can deadlock, should we remove it?
	pub fn update_with<F>(&self, f: F)
	where
		F: FnOnce(&mut P),
		P: Default,
	{
		f(self.inner.lock().progress.get_or_insert_default());
	}
}
