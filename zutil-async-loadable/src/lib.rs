//! Async loadable value.
//!
//! This crate defines a type, [`AsyncLoadable`], that can be used for
//! loading and monitoring the asynchronous loading of a value.
//!
//! It allows loading a value by spawning a [`tokio`] task, and allowing
//! progress communication from the loader. At the end, the value is available
//! from the type.

// Features
#![feature(
	async_closure,
	async_fn_traits,
	impl_trait_in_assoc_type,
	type_alias_impl_trait,
	let_chains
)]

// Modules
mod load_handle;
mod progress;

// Exports
pub use self::{
	load_handle::{LoadHandle, LoadHandleFut},
	progress::ProgressUpdater,
};

// Imports
use {
	parking_lot::Mutex,
	std::{self, error::Error, fmt, ops::AsyncFnOnce, sync::Arc},
	tokio::{sync::Notify, task},
	zutil_app_error::AppError,
};

/// Inner
pub(crate) struct Inner<T, P> {
	/// Result
	// TODO: Remove this double indirection
	res: Arc<Mutex<Option<Result<T, AppError>>>>,

	/// Progress
	progress: Mutex<Option<P>>,

	/// Task handle
	task_handle: Mutex<Option<task::AbortHandle>>,

	/// Wait
	wait: Notify,
}

/// An async fallible loadable value.
///
/// Allows the async task to communicate progress.
pub struct AsyncLoadable<T, P = ()> {
	/// Inner
	inner: Arc<Inner<T, P>>,
}

impl<T, P> AsyncLoadable<T, P> {
	/// Creates a new, unloaded, value
	pub fn new() -> Self {
		Self {
			inner: Arc::new(Inner {
				res:         Arc::new(Mutex::new(None)),
				progress:    Mutex::new(None),
				task_handle: Mutex::new(None),
				wait:        Notify::new(),
			}),
		}
	}

	/// Creates a new, loaded, value
	pub fn from_value(value: T) -> Self {
		Self {
			inner: Arc::new(Inner {
				res:         Arc::new(Mutex::new(Some(Ok(value)))),
				progress:    Mutex::new(None),
				task_handle: Mutex::new(None),
				wait:        Notify::new(),
			}),
		}
	}

	/// Creates a new, errored, value
	pub fn from_error<E>(err: &E) -> Self
	where
		E: ?Sized + Error,
	{
		Self {
			inner: Arc::new(Inner {
				res:         Arc::new(Mutex::new(Some(Err(AppError::new(&err))))),
				progress:    Mutex::new(None),
				task_handle: Mutex::new(None),
				wait:        Notify::new(),
			}),
		}
	}

	/// Gets the value of the loadable.
	pub fn get(&self) -> Option<Result<T, AppError>>
	where
		T: Clone,
	{
		self.inner.res.lock().clone()
	}

	/// Waits for this loadable to load
	///
	/// # Deadlocks
	/// If the loading task is still alive in this task when this is called,
	/// this will deadlock.
	pub async fn wait(&self) -> Result<T, AppError>
	where
		T: Clone,
	{
		#![expect(clippy::await_holding_lock, reason = "We drop the lock before `await`ing")]

		let mut res = self.inner.res.lock();
		loop {
			match &*res {
				Some(res) => break res.clone(),
				None => {
					// Get the wait future.
					// Note: According to the documentation, we do *not* need to
					//       poll it once before being added to the queue for `notify_waiters`,
					//       which we use.
					let wait_fut = self.inner.wait.notified();

					// Then await the future without the lock
					drop(res);
					wait_fut.await;
					res = self.inner.res.lock();
				},
			}
		}
	}

	/// Resets the currently loaded value.
	///
	/// Returns the old value, if any.
	pub fn reset(&self) -> Option<Result<T, AppError>> {
		self.inner.res.lock().take()
	}

	/// Gets the progress of the loadable.
	///
	/// If the progress is currently being updated, returns `None`
	pub fn progress(&self) -> Option<P>
	where
		P: Clone,
	{
		self.inner.progress.try_lock().as_deref().cloned().flatten()
	}

	/// Returns if the value is loading.
	pub fn is_loading(&self) -> bool {
		self.inner
			.task_handle
			.lock()
			.as_ref()
			.is_some_and(|task| !task.is_finished())
	}

	/// Stops the loading value.
	///
	/// If not loading, does nothing
	pub fn stop_loading(&self) {
		if let Some(task_handle) = &*self.inner.task_handle.lock() {
			task_handle.abort();
		}
	}

	/// Tries to load this value and returns a handle to get the value.
	///
	/// If already loading, returns `None`.
	///
	/// Returns a loading handle if successfully loaded.
	pub fn try_load<F>(&self, f: F) -> Option<LoadHandle<T>>
	where
		F: AsyncFnOnce(ProgressUpdater<T, P>) -> Result<T, AppError>,
		F::CallOnceFuture: Send + 'static,
		T: Send + Sync + 'static,
		P: Send + 'static,
	{
		// If we're already loading and the task isn't finished, return
		let mut task_handle = self.inner.task_handle.lock();
		if task_handle
			.as_ref()
			.is_some_and(|task_handle| !task_handle.is_finished())
		{
			return None;
		}

		// If we're already initialized, return it
		#[expect(irrefutable_let_patterns, reason = "We don't want it to live more than the if block")]
		if let res = self.inner.res.lock_arc() &&
			res.is_some()
		{
			return Some(LoadHandle::from_loaded(res));
		}

		// Otherwise start a task and return.
		let progress_updater = ProgressUpdater::new(Arc::clone(&self.inner));
		let fut = f(progress_updater);
		let join_handle = tokio::spawn({
			let inner = Arc::clone(&self.inner);
			async move {
				// Wait for the result
				// TODO: Should we catch panics here? Tokio will catch them anyway, but it
				//       might be cleaner if we also catch them and write them to the
				//       result.
				let res = fut.await;

				// Write the result
				let mut inner_res = inner.res.lock_arc();
				*inner_res = Some(res);

				// Remove the progress
				// Note: This can't deadlock, as the progress updater already exited.
				// TODO: It *might* be possible to deadlock, if `P`'s clone impl waits
				//       for the value to be loaded via `LoadHandle`, verify.
				*inner.progress.lock() = None;

				// Then wake up anyone waiting for us.
				inner.wait.notify_waiters();

				// Then hand the inner lock to the join handle
				inner_res
			}
		});
		*task_handle = Some(join_handle.abort_handle());


		Some(LoadHandle::from_task(join_handle))
	}

	/// Tries to load this value, or waits for it to be loaded.
	pub async fn try_load_or_wait<F>(&self, f: F) -> Result<T, AppError>
	where
		F: AsyncFnOnce(ProgressUpdater<T, P>) -> Result<T, AppError>,
		F::CallOnceFuture: Send + 'static,
		T: Clone + Send + Sync + 'static,
		P: Send + 'static,
	{
		// Try to load it.
		match self.try_load(f) {
			// If we managed to, await the loader handle.
			Some(load_handle) => load_handle.await,
			// Otherwise, wait for it
			None => self.wait().await,
		}
	}
}

impl<T, P> Default for AsyncLoadable<T, P> {
	fn default() -> Self {
		Self::new()
	}
}

impl<T: fmt::Debug, P: fmt::Debug> fmt::Debug for AsyncLoadable<T, P> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut f = f.debug_struct("AsyncLoadable");

		// Try to lock each field to output it
		let mut any_missing = false;
		match self.inner.res.try_lock() {
			Some(res) => _ = f.field("value", &*res),
			None => any_missing = true,
		}
		match self.inner.progress.try_lock() {
			Some(progress) => _ = f.field("progress", &*progress),
			None => any_missing = true,
		}
		match self.inner.task_handle.try_lock() {
			Some(task_handle) => {
				let is_loading = task_handle
					.as_ref()
					.is_some_and(|task_handle| !task_handle.is_finished());
				f.field("is_loading", &is_loading);
			},
			None => any_missing = true,
		}

		match any_missing {
			true => f.finish_non_exhaustive(),
			false => f.finish(),
		}
	}
}
