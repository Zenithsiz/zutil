//! Type families

/// Result family
#[sealed::sealed(erase)]
pub trait ResultFamily: Into<Result<Self::Ok, Self::Err>> + From<Result<Self::Ok, Self::Err>> {
	/// Ok type
	type Ok;

	/// Error type
	type Err;
}

#[sealed::sealed]
impl<T, E> ResultFamily for Result<T, E> {
	type Err = E;
	type Ok = T;
}

/// Tuple 2 family
#[sealed::sealed(erase)]
pub trait Tuple2Family: Into<(Self::A, Self::B)> + From<(Self::A, Self::B)> {
	/// First type
	type A;

	/// Second type
	type B;
}

#[sealed::sealed]
impl<A, B> Tuple2Family for (A, B) {
	type A = A;
	type B = B;
}

/// Slice family
#[sealed::sealed(erase)]
pub trait SliceFamily
where
	for<'a> &'a Self: From<&'a [Self::Value]> + Into<&'a [Self::Value]>,
{
	/// Value of each element
	type Value;
}

#[sealed::sealed]
impl<T> SliceFamily for [T] {
	type Value = T;
}
