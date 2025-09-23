//! Tests

// Imports
use {
	futures::FutureExt,
	std::{pin::pin, sync::Arc},
	tokio::sync::Mutex,
	app_error::AppError,
	zutil_async_loadable::AsyncLoadable,
};


#[tokio::test]
async fn load_ok() {
	let loadable = AsyncLoadable::<()>::new();

	loadable
		.try_load_or_wait(|_| async move { Ok(()) })
		.await
		.expect("Should be successful");
	assert_eq!(loadable.get(), Some(Ok(())));
}

#[tokio::test]
async fn load_err() {
	let loadable = AsyncLoadable::<()>::new();

	let err = AppError::msg("Error").context("More error");
	loadable
		.try_load_or_wait({
			let err = err.clone();
			|_| async move { Err(err) }
		})
		.await
		.expect_err("Should be error");
	assert_eq!(loadable.get(), Some(Err(err)));
}

#[tokio::test]
async fn load_loading() {
	let loadable = AsyncLoadable::<()>::new();

	let lock = Arc::new(Mutex::new(()));
	let lock_guard = lock.lock().await;

	let load_handle = loadable
		.try_load({
			let lock = Arc::clone(&lock);
			|_| async move {
				let _ = lock.lock().await;
				Ok(())
			}
		})
		.expect("Should not be loading");

	let mut load_handle_fut = pin!(load_handle.into_future());

	assert_eq!(load_handle_fut.as_mut().now_or_never(), None);
	assert_eq!(loadable.get(), None);
	assert!(loadable.is_loading());

	drop(lock_guard);

	assert_eq!(load_handle_fut.await, Ok(()));
	assert_eq!(loadable.get(), Some(Ok(())));
	assert!(!loadable.is_loading());
}
