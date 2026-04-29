//! Value

// Imports
use {
	crate::{AsNonNullOf, Base, BaseStorage, BaseVTable, ReprIs, ReprTransparent},
	core::{any::TypeId, ptr::NonNull},
};

/// A value that is part of an inheritance chain.
// TODO: Does this need to be an `unsafe trait`?
//       We encode all the layout with `ReprTransparent` and
//       `Contains`, but there could be some hidden unsoundness.
pub const trait Value: [const] ReprTransparent<Inner = Base> + Sized + 'static {
	/// Fields
	type Fields;

	/// Methods
	type Methods;

	/// Storage.
	type Storage: [const] ReprIs<<Self::Parent as Value>::Storage>
		+ [const] ReprIs<BaseStorage>
		+ [const] AsRef<Self::Fields>
		+ [const] AsNonNullOf<Self::Fields>;

	/// `VTable`
	type VTable: [const] ReprIs<<Self::Parent as Value>::VTable>
		+ [const] ReprIs<BaseVTable>
		+ [const] AsRef<Self::Methods>
		+ [const] AsNonNullOf<Self::Methods>;

	/// Parent value
	type Parent: ValueFor<Self>;

	/// Parents of this type.
	const PARENTS: &'static [TypeId];

	/// `VTable` of this type
	const VTABLE: &'static Self::VTable;

	/// Returns a reference to the storage
	fn storage(&self) -> &Self::Storage {
		let base = Base::from_value_ref(self);

		// SAFETY: `Self` always has a valid `Self::Storage`.
		unsafe { base.storage_of::<Self>() }
	}

	/// Returns a reference to the fields
	fn fields(&self) -> &Self::Fields {
		let base = Base::from_value_ref(self);

		// SAFETY: `Self` always has a valid `Self::Storage`.
		unsafe { base.fields_of::<Self>() }
	}

	/// Returns a reference to the vtable
	fn vtable(&self) -> &'static Self::VTable {
		let base = Base::from_value_ref(self);

		// SAFETY: `Self` always has a valid `Self::Storage`.
		unsafe { base.vtable_of::<Self>() }
	}

	/// Creates this value from it's storage
	fn from_storage(storage: Self::Storage) -> Self {
		let base = Base::new::<Self>(storage);

		// SAFETY: The value was created with `Self::Storage`,
		//         and is thus valid for `Self`.
		unsafe { Self::from_repr(base) }
	}

	/// Creates this value from it's storage pointer
	///
	/// # Safety
	/// You must ensure that `storage_ptr` contains a valid instance
	/// of `T::Storage` and was allocated with [`Global`] with the
	/// layout of `T::Storage`.
	#[must_use]
	unsafe fn from_storage_ptr(storage_ptr: NonNull<Self::Storage>) -> Self {
		// SAFETY: Caller ensures `storage_ptr` invariants.
		let base = unsafe { Base::from_storage_ptr_of::<Self>(storage_ptr) };

		// SAFETY: Caller ensures the value was created with `Self::Storage`,
		//         and is thus valid for `Self`.
		unsafe { Self::from_repr(base) }
	}
}

/// Trait implemented for types that allow `T` as a child value
pub trait ValueFor<T>: Value {}
