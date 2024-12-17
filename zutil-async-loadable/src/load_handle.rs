//! Loading handle

// Imports
use {
	parking_lot::ArcMutexGuard,
	std::{
		future::{Future, IntoFuture},
		pin::Pin,
		task::Poll,
	},
	tokio::task,
	zutil_app_error::AppError,
};

/// Load handle inner
enum LoaderHandleInner<T> {
	/// Task
	Task(task::JoinHandle<ArcMutexGuard<parking_lot::RawMutex, Option<Result<T, AppError>>>>),

	/// Already loaded
	Loaded(ArcMutexGuard<parking_lot::RawMutex, Option<Result<T, AppError>>>),
}

/// Load handle
pub struct LoadHandle<T> {
	/// Inner
	inner: LoaderHandleInner<T>,

	/// Whether to abort the loading when this handle's future is cancelled
	abort_on_drop: bool,
}

impl<T> LoadHandle<T> {
	/// Creates the loader handle
	fn new(inner: LoaderHandleInner<T>) -> Self {
		Self {
			inner,
			abort_on_drop: true,
		}
	}

	/// Creates a loader handle from a task
	pub(crate) fn from_task(
		task: task::JoinHandle<ArcMutexGuard<parking_lot::RawMutex, Option<Result<T, AppError>>>>,
	) -> Self {
		Self::new(LoaderHandleInner::Task(task))
	}

	/// Creates a loader handle from a loaded value
	pub(crate) fn from_loaded(res: ArcMutexGuard<parking_lot::RawMutex, Option<Result<T, AppError>>>) -> Self {
		Self::new(LoaderHandleInner::Loaded(res))
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
pub struct LoadHandleFut<T = ()>
where
	T: Clone,
{
	/// Inner future
	#[pin]
	inner: load_handle_fut_inner::Fut<T>,

	/// Abort on drop.
	// Note: It's fine to unconditionally drop this, even after the task
	//       is completed, since that will just do nothing.
	abort_on_drop: Option<AbortTaskOnDrop>,
}

impl<T> Future for LoadHandleFut<T>
where
	T: Clone,
{
	type Output = Result<T, AppError>;

	fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
		self.project().inner.poll(cx)
	}
}

impl<T> IntoFuture for LoadHandle<T>
where
	T: Clone,
{
	type IntoFuture = LoadHandleFut<T>;
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
	use {super::*, zutil_app_error::app_error};

	/// The inner future
	pub type Fut<T>
	where
		T: Clone,
	= impl Future<Output = Result<T, AppError>>;

	/// Creates the inner future
	pub fn new<T>(inner: LoaderHandleInner<T>) -> Fut<T>
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
			inner.clone().expect("Value should be loaded")
		}
	}
}
