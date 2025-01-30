//! Error pretty printing

// Imports
use {
	crate::{AppError, Inner},
	core::fmt,
	itertools::{Itertools, Position as ItertoolsPos},
	std::vec,
};

/// Pretty display for [`AppError`]
#[derive(Debug)]
pub struct PrettyDisplay<'a> {
	/// Root error
	root: &'a AppError,
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum Column {
	Line,
	Empty,
}

impl Column {
	/// Returns the string for this column
	const fn as_str(self) -> &'static str {
		match self {
			Self::Line => "│ ",
			Self::Empty => "  ",
		}
	}
}

impl<'a> PrettyDisplay<'a> {
	/// Creates a new pretty display
	pub(crate) fn new(root: &'a AppError) -> Self {
		Self { root }
	}

	/// Formats a single error
	fn fmt_single(&self, f: &mut fmt::Formatter<'_>, err: &AppError, columns: &mut Vec<Column>) -> fmt::Result {
		// If it's multiple, display it as multiple
		let (msg, source) = match &*err.inner {
			Inner::Single { msg, source } => (msg, source),
			Inner::Multiple(errs) => return self.fmt_multiple(f, errs, columns),
		};

		// Else write the top-level error
		write!(f, "{msg}")?;

		// Then, if there's a cause, write the rest
		if let Some(mut cur_source) = source.as_ref() {
			let starting_columns = columns.len();
			loop {
				// Print the pre-amble
				f.pad("\n")?;
				for c in &*columns {
					f.pad(c.as_str())?;
				}
				f.pad("└─")?;
				columns.push(Column::Empty);

				// Then check if we got to a multiple.
				match &*cur_source.inner {
					Inner::Single { msg, source } => {
						write!(f, "{msg}",)?;

						// And descend
						cur_source = match source {
							Some(source) => source,
							_ => break,
						};
					},
					Inner::Multiple(errs) => {
						self.fmt_multiple(f, errs, columns)?;
						break;
					},
				}
			}
			let _: vec::Drain<'_, _> = columns.drain(starting_columns..);
		}

		Ok(())
	}

	/// Formats multiple errors
	fn fmt_multiple(&self, f: &mut fmt::Formatter<'_>, errs: &[AppError], columns: &mut Vec<Column>) -> fmt::Result {
		// Write the top-level error
		write!(f, "Multiple errors:")?;

		// For each error, write it
		for (pos, err) in errs.iter().with_position() {
			f.pad("\n")?;
			for c in &*columns {
				f.pad(c.as_str())?;
			}

			match matches!(pos, ItertoolsPos::Last | ItertoolsPos::Only) {
				true => {
					f.pad("└─")?;
					columns.push(Column::Empty);
				},
				false => {
					f.pad("├─")?;
					columns.push(Column::Line);
				},
			}

			self.fmt_single(f, err, columns)?;
			let _: Option<_> = columns.pop();
		}

		Ok(())
	}
}

impl fmt::Display for PrettyDisplay<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut columns = vec![];
		self.fmt_single(f, self.root, &mut columns)?;
		assert_eq!(columns.len(), 0, "There should be no columns after formatting");

		Ok(())
	}
}
