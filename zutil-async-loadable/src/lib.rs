//! Async loadable value.
//!
//! This crate defines a type, [`AsyncLoadable`], that can be used for
//! loading and monitoring the asynchronous loading of a value.
//!
//! It allows loading a value by spawning a [`tokio`] task, and allowing
//! progress communication from the loader. At the end, the value is available
//! from the type.

// Features
#![feature(async_closure, async_fn_traits, impl_trait_in_assoc_type)]

// Imports
use parking_lot::{ArcMutexGuard, Mutex};
use std::{
	fmt,
	future::{Future, IntoFuture},
	ops::AsyncFnOnce,
	sync::Arc,
};
use tokio::{sync::Notify, task};
use zutil_app_error::{AppError, app_error};

/// Inner
struct Inner<T, P> {
	/// Result
	res: Option<Result<T, AppError>>,

	/// Progress
	progress: Option<P>,

	/// Task handle
	task_handle: Option<task::AbortHandle>,

	/// Wait
	wait: Arc<Notify>,
}

/// An async fallible loadable value.
///
/// Allows the async task to communicate progress.
pub struct AsyncLoadable<T, P = ()> {
	/// Inner
	inner: Arc<Mutex<Inner<T, P>>>,
}

impl<T, P> AsyncLoadable<T, P> {
	/// Creates a new, unloaded, value
	pub fn new() -> Self {
		Self {
			inner: Arc::new(Mutex::new(Inner {
				res:         None,
				progress:    None,
				task_handle: None,
				wait:        Arc::new(Notify::new()),
			})),
		}
	}

	/// Gets the value of the loadable.
	pub fn get(&self) -> Option<Result<T, AppError>>
	where
		T: Clone,
	{
		self.inner.lock().res.clone()
	}

	/// Waits for this loadable to load
	pub async fn wait(&self) -> Result<T, AppError>
	where
		T: Clone,
	{
		#![expect(clippy::await_holding_lock, reason = "We drop the lock before `await`ing")]

		// Otherwise, wait until we're notified by the task
		let mut inner = self.inner.lock();
		loop {
			match &inner.res {
				Some(res) => break res.clone(),
				None => {
					// Get the wait future.
					// Note: According to the documentation, we do *not* need to
					//       poll it once before being added to the queue for `notify_waiters`,
					//       which we use.
					let wait = Arc::clone(&inner.wait);
					let wait_fut = wait.notified();

					// Then await the future without the lock
					drop(inner);
					wait_fut.await;
					inner = self.inner.lock();
				},
			}
		}
	}

	/// Resets the currently loaded value.
	///
	/// Returns the old value, if any.
	pub fn reset(&self) -> Option<Result<T, AppError>> {
		self.inner.lock().res.take()
	}

	/// Gets the progress of the loadable.
	pub fn progress(&self) -> Option<P>
	where
		P: Clone,
	{
		self.inner.lock().progress.clone()
	}

	/// Returns if the value is loading.
	pub fn is_loading(&self) -> bool {
		self.inner
			.lock()
			.task_handle
			.as_ref()
			.is_some_and(|task| !task.is_finished())
	}

	/// Stops the loading value.
	///
	/// If not loading, does nothing
	pub fn stop_loading(&self) {
		let inner = self.inner.lock();
		if let Some(task_handle) = &inner.task_handle {
			task_handle.abort();
		}
	}

	/// Tries to load this value and returns a handle to get the value.
	///
	/// If already loading, returns `None`.
	///
	/// Returns a loading handle if successfully loaded.
	pub fn try_load<F>(&self, f: F) -> Option<LoadHandle<T, P>>
	where
		F: AsyncFnOnce(ProgressUpdater<T, P>) -> Result<T, AppError>,
		F::CallOnceFuture: Send + 'static,
		T: Send + Sync + 'static,
		P: Send + 'static,
	{
		// If we're already loading and the task isn't finished, return
		let mut inner = self.inner.lock_arc();
		if inner.task_handle.as_ref().is_some_and(|task| !task.is_finished()) {
			return None;
		}

		// If we're already initialized, return it
		if inner.res.is_some() {
			return Some(LoadHandle {
				inner: LoaderHandleInner::Loaded(inner),
			});
		}

		// Otherwise start a task and return.
		let fut = f(ProgressUpdater {
			inner: Arc::clone(&self.inner),
		});
		let join_handle = tokio::spawn({
			let inner = Arc::clone(&self.inner);
			async move {
				// Wait for the result
				// TODO: Should we catch panics here? Tokio will catch them anyway, but it
				//       might be cleaner if we also catch them and write them to the
				//       result.
				let res = fut.await;

				// Write the result, remove the progress and notify everyone
				let mut inner = inner.lock_arc();
				inner.res = Some(res);
				inner.progress = None;
				inner.wait.notify_waiters();

				// Then hand the inner lock to the join handle
				inner
			}
		});
		inner.task_handle = Some(join_handle.abort_handle());


		Some(LoadHandle {
			inner: LoaderHandleInner::Task(join_handle),
		})
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
		// Try to lock inner, or just display nothing
		let Some(inner) = self.inner.try_lock() else {
			return f.debug_struct("AsyncLoadable").finish_non_exhaustive();
		};

		let is_loading = inner.task_handle.as_ref().is_some_and(|task| !task.is_finished());
		f.debug_struct("AsyncLoadable")
			.field("value", &inner.res)
			.field("progress", &inner.progress)
			.field("is_loading", &is_loading)
			.finish()
	}
}

/// Load handle inner
enum LoaderHandleInner<T, P> {
	/// Task
	Task(task::JoinHandle<ArcMutexGuard<parking_lot::RawMutex, Inner<T, P>>>),

	/// Already loaded
	Loaded(ArcMutexGuard<parking_lot::RawMutex, Inner<T, P>>),
}

/// Load handle
pub struct LoadHandle<T, P = ()> {
	/// Inner
	inner: LoaderHandleInner<T, P>,
}

impl<T, P> IntoFuture for LoadHandle<T, P>
where
	T: Clone,
{
	type Output = Result<T, AppError>;

	type IntoFuture = impl Future<Output = Self::Output>;

	fn into_future(self) -> Self::IntoFuture {
		async move {
			// Get the lock to inner
			let inner = match self.inner {
				LoaderHandleInner::Task(join_handle) => {
					join_handle.await.map_err(|err| match err.try_into_panic() {
						Ok(err) => app_error!("Loader panicked: {err:?}"),
						Err(err) => AppError::new(&err).context("Loader was cancelled"),
					})?
				},
				LoaderHandleInner::Loaded(inner) => inner,
			};

			// Then get the value
			inner.res.clone().expect("Value should be loaded")
		}
	}
}

/// Progress updater
pub struct ProgressUpdater<T, P> {
	/// Inner
	inner: Arc<Mutex<Inner<T, P>>>,
}

impl<T, P> ProgressUpdater<T, P> {
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
