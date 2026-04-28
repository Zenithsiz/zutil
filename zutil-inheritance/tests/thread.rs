//! Thread tests

// Features
#![feature(const_trait_impl, more_qualified_paths, trivial_bounds)]

// Imports
use {std::thread, zutil_inheritance::FromFields};

zutil_inheritance::value! {
	struct A(): Send + Sync {}
	impl Self {}
}

zutil_inheritance::value! {
	struct B(A): Send + Sync {}
	impl Self {}
}

zutil_inheritance::value! {
	struct C(B, A): Send + Sync {}
	impl Self {}
}

#[test]
fn drop_on_other_thread() {
	let a = A::from_fields((AFields {},));
	thread::spawn(|| {
		let _a: A = a;
	});
}
