//! Multiple error handling

// Imports
use {
	crate::AppError,
	itertools::Itertools,
	std::ops::{ControlFlow, FromResidual, Try},
};

/// Helper type to collect a `IntoIter<Item = Result<T, AppError>>`
/// into a `Result<C, AppError>` with all of the errors instead of the first.
#[derive(Debug)]
pub enum AllErrs<C> {
	Ok(C),
	Err(Vec<AppError>),
}

impl<C> Default for AllErrs<C>
where
	C: Default,
{
	fn default() -> Self {
		Self::Ok(C::default())
	}
}
impl<C, U> Extend<Result<U, AppError>> for AllErrs<C>
where
	C: Extend<U>,
{
	fn extend<T: IntoIterator<Item = Result<U, AppError>>>(&mut self, iter: T) {
		// TODO: Do this more efficiently?
		for res in iter {
			match (&mut *self, res) {
				// If we have a collection, and we get an item, extend it
				(Self::Ok(collection), Ok(item)) => collection.extend_one(item),
				// If we have a collection, but find an error, switch to errors
				(Self::Ok(_), Err(err)) => *self = Self::Err(vec![err]),
				// If we have errors and got an item, ignore it
				(Self::Err(_), Ok(_)) => (),
				// If we have errors and got an error, extend it
				(Self::Err(errs), Err(err)) => errs.push(err),
			}
		}
	}
}

impl<C, T> FromIterator<Result<T, AppError>> for AllErrs<C>
where
	C: Default + Extend<T>,
{
	fn from_iter<I>(iter: I) -> Self
	where
		I: IntoIterator<Item = Result<T, AppError>>,
	{
		// TODO: If we get any errors, don't allocate memory for the rest of the values?
		let (values, errs) = iter.into_iter().partition_result::<C, Vec<_>, _, _>();
		match errs.is_empty() {
			true => Self::Ok(values),
			false => Self::Err(errs),
		}
	}
}

#[derive(Debug)]
pub struct AllErrsResidue(Vec<AppError>);

impl<C> Try for AllErrs<C> {
	type Output = C;
	type Residual = AllErrsResidue;

	fn from_output(output: Self::Output) -> Self {
		Self::Ok(output)
	}

	fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
		match self {
			Self::Ok(values) => ControlFlow::Continue(values),
			Self::Err(errs) => ControlFlow::Break(AllErrsResidue(errs)),
		}
	}
}

impl<T> FromResidual<AllErrsResidue> for AllErrs<T> {
	fn from_residual(residual: AllErrsResidue) -> Self {
		Self::Err(residual.0)
	}
}

impl<T> FromResidual<AllErrsResidue> for Result<T, AppError> {
	fn from_residual(residual: AllErrsResidue) -> Self {
		let err = match <[_; 1]>::try_from(residual.0) {
			Ok([err]) => err,
			Err(errs) => {
				assert!(!errs.is_empty(), "`ResultMultipleResidue` should hold at least 1 error");
				AppError::from_multiple(errs)
			},
		};

		Err(err)
	}
}
