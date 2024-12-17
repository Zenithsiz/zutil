//! Loading handle

// Imports
use {
	crate::Inner,
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
	pub(crate) fn from_task(task: task::JoinHandle<ArcMutexGuard<parking_lot::RawMutex, Inner<T, P>>>) -> Self {
		Self::new(LoaderHandleInner::Task(task))
	}

	/// Creates a loader handle from a loaded value
	pub(crate) fn from_loaded(value: ArcMutexGuard<parking_lot::RawMutex, Inner<T, P>>) -> Self {
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
	use {super::*, zutil_app_error::app_error};

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
