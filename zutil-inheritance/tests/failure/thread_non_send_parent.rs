#![feature(
	macro_metavar_expr,
	macro_metavar_expr_concat,
	const_trait_impl,
	const_index,
	const_cmp,
	more_qualified_paths,
	trivial_bounds,
	unsize
)]

// Imports
use {
	zutil_inheritance::Value,
	std::{cell::RefCell, thread},
};

zutil_inheritance::value! {
	struct A() {
		a: RefCell<u32>,
	}
	impl Self {}
}

zutil_inheritance::value! {
	struct B(A): Send + Sync {}
	impl Self {}
}

fn send_b(b: B) {
	thread::spawn(move || {
		let _ = (*b).fields().a.borrow_mut();
	});
}

fn main() {}
