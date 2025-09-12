//! Trailing comma test

// Features
#![feature(proc_macro_hygiene, stmt_expr_attributes)]

// Imports
use zutil_cloned::cloned;

fn single(_: String) {}
fn multi(_: i32, _: String, _: i32) {}

#[test]
fn trailing_comma() {
	let a = String::new();

	multi(
		1,
		#[cloned(a)]
		a,
		5,
	);

	single(
		#[cloned(a)]
		a,
	);
}
