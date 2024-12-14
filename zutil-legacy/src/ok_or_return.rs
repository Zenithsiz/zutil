//! `Ok` or return a value

use std::ops::{ControlFlow, FromResidual, Try};

/// Extension trait to create a [`OkOrReturnResult`]
pub trait OkOrReturn: Try + Sized {
	/// Returns the output of this result, or returns `value`
	fn ok_or_return<Ret>(self, value: Ret) -> OkOrReturnResult<Self::Output, Ret> {
		self.ok_or_else_return(|_| value)
	}

	/// Returns the output of this result, or returns with the output of `f`
	fn ok_or_else_return<Ret, F: FnOnce(Self::Residual) -> Ret>(self, f: F) -> OkOrReturnResult<Self::Output, Ret>;
}

impl<T: Try> OkOrReturn for T {
	fn ok_or_else_return<Ret, F: FnOnce(Self::Residual) -> Ret>(self, f: F) -> OkOrReturnResult<Self::Output, Ret> {
		match self.branch() {
			ControlFlow::Continue(output) => OkOrReturnResult::Ok(output),
			ControlFlow::Break(residual) => OkOrReturnResult::Ret(f(residual)),
		}
	}
}

/// `Try` type for getting either a value out, or returning a value
pub enum OkOrReturnResult<T, Ret> {
	/// Successful
	Ok(T),

	/// Return
	Ret(Ret),
}

/// Residual for [`OkOrReturnResult`]
pub struct OkOrReturnResidual<Ret> {
	/// Return value
	ret: Ret,
}

impl<T, Ret> Try for OkOrReturnResult<T, Ret> {
	type Output = T;
	type Residual = OkOrReturnResidual<Ret>;

	fn from_output(output: Self::Output) -> Self {
		Self::Ok(output)
	}

	fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
		match self {
			Self::Ok(output) => ControlFlow::Continue(output),
			Self::Ret(ret) => ControlFlow::Break(OkOrReturnResidual { ret }),
		}
	}
}

impl<Ret> FromResidual<OkOrReturnResidual<Ret>> for Ret {
	fn from_residual(residual: OkOrReturnResidual<Ret>) -> Self {
		residual.ret
	}
}

impl<T, Ret> FromResidual<OkOrReturnResidual<Ret>> for OkOrReturnResult<T, Ret> {
	fn from_residual(residual: OkOrReturnResidual<Ret>) -> Self {
		OkOrReturnResult::Ret(residual.ret)
	}
}
