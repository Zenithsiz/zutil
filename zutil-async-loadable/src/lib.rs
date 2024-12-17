//! Async loadable value.
//!
//! This crate defines a type, [`AsyncLoadable`], that can be used for
//! loading and monitoring the asynchronous loading of a value.
//!
//! It allows loading a value by spawning a [`tokio`] task, and allowing
//! progress communication from the loader. At the end, the value is available
//! from the type.

// Features
#![feature(async_closure, async_fn_traits, impl_trait_in_assoc_type, type_alias_impl_trait)]

// Modules
mod progress;

// Exports
pub use progress::ProgressUpdater;

// Imports
use {
	parking_lot::{ArcMutexGuard, Mutex},
	std::{
		fmt,
		future::{Future, IntoFuture},
		ops::AsyncFnOnce,
		pin::Pin,
		sync::Arc,
		task::Poll,
	},
	tokio::{sync::Notify, task},
	zutil_app_error::{AppError, app_error},
};

/// Inner
pub(crate) struct Inner<T, P> {
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
			return Some(LoadHandle::from_loaded(inner));
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

	/// Whether to abort the loading when this handle's future is cancelled
	abort_on_drop: bool,
}

impl<T, P> LoadHandle<T, P> {
	/// Creates the loader handle
	fn new(inner: LoaderHandleInner<T, P>) -> Self {
		Self {
			inner,
			abort_on_drop: true,
		}
	}

	/// Creates a loader handle from a task
	fn from_task(task: task::JoinHandle<ArcMutexGuard<parking_lot::RawMutex, Inner<T, P>>>) -> Self {
		Self::new(LoaderHandleInner::Task(task))
	}

	/// Creates a loader handle from a loaded value
	fn from_loaded(value: ArcMutexGuard<parking_lot::RawMutex, Inner<T, P>>) -> Self {
		Self::new(LoaderHandleInner::Loaded(value))
	}

	/// Sets whether the inner task should be aborted if this handle's
	/// future is dropped.
	///
	/// By default, this is `true`
	pub fn with_abort_on_drop(self, abort_on_drop: bool) -> Self {
		Self { abort_on_drop, ..self }
	}
}

/// Abort task on drop
struct AbortTaskOnDrop {
	/// Task handle
	task_handle: task::AbortHandle,
}

impl Drop for AbortTaskOnDrop {
	fn drop(&mut self) {
		self.task_handle.abort();
	}
}

/// Load handle future
#[pin_project::pin_project]
pub struct LoadHandleFut<T, P = ()>
where
	T: Clone,
{
	/// Inner future
	#[pin]
	inner: load_handle_fut_inner::Fut<T, P>,

	/// Abort on drop.
	// Note: It's fine to unconditionally drop this, even after the task
	//       is completed, since that will just do nothing.
	abort_on_drop: Option<AbortTaskOnDrop>,
}

impl<T, P> Future for LoadHandleFut<T, P>
where
	T: Clone,
{
	type Output = Result<T, AppError>;

	fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
		self.project().inner.poll(cx)
	}
}

impl<T, P> IntoFuture for LoadHandle<T, P>
where
	T: Clone,
{
	type IntoFuture = LoadHandleFut<T, P>;
	type Output = Result<T, AppError>;

	fn into_future(self) -> Self::IntoFuture {
		let abort_on_drop = match self.abort_on_drop {
			true => match &self.inner {
				LoaderHandleInner::Task(join_handle) => Some(AbortTaskOnDrop {
					task_handle: join_handle.abort_handle(),
				}),
				LoaderHandleInner::Loaded(_) => None,
			},
			false => None,
		};

		LoadHandleFut {
			inner: load_handle_fut_inner::new(self.inner),
			abort_on_drop,
		}
	}
}

mod load_handle_fut_inner {
	use super::*;

	/// The inner future
	pub type Fut<T, P>
	where
		T: Clone,
	= impl Future<Output = Result<T, AppError>>;

	/// Creates the inner future
	pub fn new<T, P>(inner: LoaderHandleInner<T, P>) -> Fut<T, P>
	where
		T: Clone,
	{
		async move {
			// Get the lock to inner
			let inner = match inner {
				LoaderHandleInner::Task(join_handle) =>
					join_handle.await.map_err(|err| match err.try_into_panic() {
						Ok(err) => app_error!("Loader panicked: {err:?}"),
						Err(err) => AppError::new(&err).context("Loader was cancelled"),
					})?,
				LoaderHandleInner::Loaded(inner) => inner,
			};

			// Then get the value
			inner.res.clone().expect("Value should be loaded")
		}
	}
}
