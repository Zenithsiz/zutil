#![feature(
	macro_metavar_expr,
	macro_metavar_expr_concat,
	const_trait_impl,
	const_index,
	const_cmp,
	more_qualified_paths,
	trivial_bounds,
	unsize,
	macro_derive
)]

zutil_inheritance::value! {
	struct A() {}
	impl Self {}
}

zutil_inheritance::value! {
	struct B(A) {}
	impl Self {}
}

zutil_inheritance::value! {
	struct C(A) {}
	impl Self {}
}

zutil_inheritance::value! {
	struct D(B, C) {}
	impl Self {}
}

fn main() {}
