//! Object extension

// Imports
use {
	crate::{AsNonNullOf, Base, ReprTransparent, Value},
	core::alloc::Layout,
	std::alloc::{Allocator, Global},
};

/// Extends this object to include a parent
pub trait Extend: Value {
	fn extend_with_fields<T: Value<Parent = Self>>(self, fields: T::Fields) -> Result<T, Self>;
}

impl<T: Value> Extend for T {
	fn extend_with_fields<U: Value<Parent = Self>>(self, fields: U::Fields) -> Result<U, Self> {
		let base = Base::from_value(self);
		let old_layout = base.vtable().storage_layout;
		let storage_ptr = match base.into_storage_ptr_of::<Self>() {
			Ok(storage) => storage,
			// SAFETY: The value is valid since we just got it from `self`
			Err(base) => return Err(unsafe { ReprTransparent::from_repr(base) }),
		};

		let new_layout = Layout::new::<U::Storage>();
		// SAFETY: `storage_ptr` was allocated with `Global` and `old_layout`.
		//         Since `U`'s parent is `Self`, we only add a single uninitialized part
		//         of `U::Fields`.
		let storage_ptr = match unsafe { Global.grow(storage_ptr.cast(), old_layout, new_layout) } {
			Ok(ptr) => ptr.cast::<U::Storage>(),
			Err(_) => panic!("Unable to re-allocate storage"),
		};

		let storage_fields = AsNonNullOf::<U::Fields>::as_non_null_of(storage_ptr);
		// SAFETY: We just allocated space for a `U::Storage` and got it's fields.
		unsafe { storage_fields.write(fields) };

		// SAFETY: `storage_ptr` is a valid pointer allocated with `Global`.
		let value = unsafe { U::from_storage_ptr(storage_ptr) };
		Ok(value)
	}
}
