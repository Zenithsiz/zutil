//! Inheritance.
//!
//! Implements an inheritance system in rust.
//!
//! See the [`value`] macro for details on how to create new
//! types in an inheritance chain.
//!
//! # Examples
//!
//! ```rust
//! #![feature(const_trait_impl, trivial_bounds, more_qualified_paths)]
//!
//! zutil_inheritance::value! {
//! 	struct Parent() {}
//! 	impl Self {}
//! }
//!
//! zutil_inheritance::value! {
//! 	struct Child(Parent) {}
//! 	impl Self {}
//! }
//!
//! use zutil_inheritance::{FromFields, Downcast};
//!
//! let child = Child::from_fields((ChildFields {}, ParentFields {}));
//! let child_as_parent = Parent::from(child);
//! let child = child_as_parent.downcast::<Child>().expect("Should be a `Child`");
//! ```

// Features
#![feature(
	const_trait_impl,
	const_convert,
	const_heap,
	const_cmp,
	const_clone,
	allocator_api,
	try_as_dyn
)]

// Modules
mod base;
mod clone_storage;
mod debug;
mod downcast;
mod extend;
mod ref_count;
mod storage;
mod util;
mod value;
mod vtable;
mod weak;

// Exports
pub use {
	self::{
		base::Base,
		clone_storage::CloneStorage,
		debug::DebugFields,
		downcast::Downcast,
		extend::Extend,
		storage::BaseStorage,
		util::{AsNonNullOf, ReprIs, ReprTransparent},
		value::{Value, ValueFor},
		vtable::BaseVTable,
		weak::{ValueDowngrade, WeakValue},
	},
	zutil_inheritance_macros::value,
};

/// Creates a value from it's fields.
pub const trait FromFields {
	type Fields;

	fn from_fields(fields: Self::Fields) -> Self;
}

/// Creates a storage from it's fields
pub const trait StorageFromFields {
	type Fields;

	fn from_fields(base: BaseStorage, fields: Self::Fields) -> Self;
}

/// Creates a vtable from it's methods
pub const trait VTableFromMethods {
	type Methods;

	fn from_methods(base: BaseVTable, methods: Self::Methods) -> Self;
}
