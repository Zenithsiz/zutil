//! Trailing semicolon test

// Features
#![feature(proc_macro_hygiene, stmt_expr_attributes)]

// Imports
use zutil_cloned::cloned;

fn call(_: String) -> i32 {
	5
}

#[test]
fn trailing_semi() {
	let a = String::new();

	let _: String = {
		#[cloned(a)]
		a
	};

	#[expect(path_statements)]
	let _: () = {
		// TODO: Not require trailing `;` once `stmt_expr_attributes` lets us
		//       parse the trailing semicolon ourselves
		#[cloned(a;)]
		a;
	};

	let _: () = {
		#[cloned(a;)]
		call(a);
	};
}
