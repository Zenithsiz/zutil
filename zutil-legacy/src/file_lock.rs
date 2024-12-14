//! File lock

// Imports
use std::{
	fs, io,
	path::{Path, PathBuf},
};

/// A file lock
#[derive(Debug)]
pub struct FileLock {
	/// Lock path
	path: PathBuf,
}

impl FileLock {
	/// Creates a new file lock
	pub fn new(path: impl Into<PathBuf> + AsRef<Path>) -> Option<Self> {
		// Then try to open it
		fs::OpenOptions::new()
			.write(true)
			.create_new(true)
			.open(&path)
			.map(move |_| Self { path: path.into() })
			.ok()
	}

	/// Unlocks this lock.
	///
	/// Note: This should only be called right before destroying the file lock
	fn unlock_ref_mut(&mut self) -> Result<(), io::Error> {
		// Try to delete the file
		fs::remove_file(&self.path)?;

		Ok(())
	}

	/// Unlocks this lock
	pub fn unlock(mut self) -> Result<(), io::Error> {
		// Unlock ourselves
		// Note: We can't use `?`, as then we'd also run the destructor if it failed.
		let res = self.unlock_ref_mut();

		// And forget ourselves
		#[allow(clippy::mem_forget)] // We explicitly do not want to run the destructor
		std::mem::forget(self);

		res
	}
}

impl Drop for FileLock {
	fn drop(&mut self) {
		if let Err(err) = self.unlock_ref_mut() {
			log::warn!("Unable to unlock {:?}: {err}", self.path);
		}
	}
}
