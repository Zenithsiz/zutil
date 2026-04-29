//! Utilities

// Imports
use core::{
	mem,
	ptr::{self, NonNull},
};

/// Marker trait for types that contain a type `T`
/// at offset 0.
///
/// # Safety
/// You must ensure that `Self` contains a type `T`
/// at offset 0, with suitable alignment.
pub const unsafe trait ReprIs<T>: Sized {
	/// Casts a `NonNull<Self>` to `NonNull<T>`
	#[must_use]
	fn to_non_null(ptr: NonNull<Self>) -> NonNull<T> {
		ptr.cast()
	}

	/// Casts a `NonNull<T>` to `NonNull<Self>`
	#[must_use]
	fn from_non_null(ptr: NonNull<T>) -> NonNull<Self> {
		ptr.cast()
	}
}

// SAFETY: `T` always contains itself at offset 0
unsafe impl<T> const ReprIs<T> for T {}

/// Marker trait for types that are `repr(transparent)`.
///
/// # Safety
/// You must ensure that `Self` is `repr(transparent)`.
/// It is also fine to implement this for `Self`.
pub const unsafe trait ReprTransparent: Sized {
	/// Inner type that we're `repr(transparent)` over
	type Inner;

	/// Casts a `&Self::Inner` to `&Self`
	#[must_use]
	fn to_ref(ref_: &Self) -> &Self::Inner {
		// SAFETY: Implementor ensures that `Self` is `repr(transparent)`.
		unsafe { ptr::from_ref(ref_).cast::<Self::Inner>().as_ref_unchecked() }
	}

	/// Casts a `&Self` to `&Self::Inner`
	///
	/// # Safety
	/// You must ensure that the value of `Self::Inner` is valid for `Self`
	#[must_use]
	unsafe fn from_ref(ref_: &Self::Inner) -> &Self {
		// SAFETY: Implementor ensures that `Self` is `repr(transparent)`,
		//         and caller ensures that `ref_` is a valid instance of `Self`.
		unsafe { ptr::from_ref(ref_).cast::<Self>().as_ref_unchecked() }
	}

	/// Converts a `Self::Inner` to `Self`
	///
	/// # Safety
	/// You must ensure that the value of `Self::Inner` is valid for `Self`
	unsafe fn from_repr(value: Self::Inner) -> Self {
		// SAFETY: Implementor ensures that `Self` is `repr(transparent)`.
		//         Caller ensures that `value` is a valid instance of `Self`.
		let output = unsafe { mem::transmute_copy::<Self::Inner, Self>(&value) };
		mem::forget(value);

		output
	}

	/// Converts a `Self` to `Self::Inner`
	fn into_repr(self) -> Self::Inner {
		// SAFETY: Implementor ensures that `Self` is `repr(transparent)`.
		let value = unsafe { mem::transmute_copy::<Self, Self::Inner>(&self) };
		mem::forget(self);

		value
	}
}

#[macro_export]
macro_rules! ReprTransparent {
	derive() (
		#[repr(transparent)]
		$v:vis struct $Ty:ident($Inner:ty);
	) => {
		unsafe impl const zutil_inheritance::ReprTransparent for $Ty {
			type Inner = $Inner;
		}
	}
}

/// [`AsRef`] for [`NonNull`] pointers.
pub const trait AsNonNullOf<T>: Sized {
	/// Gets the pointer to a `T` from `Self`
	#[must_use]
	fn as_non_null_of(this: NonNull<Self>) -> NonNull<T>;
}
