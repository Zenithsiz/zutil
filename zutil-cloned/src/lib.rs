//! Macros for `zutil-cloned`

// Features
#![feature(if_let_guard, try_blocks)]

// Imports
use {
	core::iter,
	proc_macro::TokenStream,
	quote::ToTokens,
	syn::{punctuated::Punctuated, spanned::Spanned},
};

#[proc_macro_attribute]
pub fn cloned(attr: TokenStream, input: TokenStream) -> TokenStream {
	let attrs = syn::parse_macro_input!(attr as Attrs);
	let mut input = syn::parse_macro_input!(input as Input);

	let clones = attrs
		.clones
		.into_iter()
		.map(|attr| {
			let ident = attr.ident;
			let expr = match attr.expr {
				Some(expr) => expr,
				None => self::ident_to_expr(ident.clone()),
			};
			syn::parse_quote! {
				let #ident = #expr.clone();
			}
		})
		.collect::<Vec<syn::Stmt>>();

	// Wraps the expression in the clones
	let wrap_expr = |expr: &mut syn::Expr, trailing_semi: Option<syn::Token![;]>| {
		*expr = syn::parse_quote_spanned! {expr.span() => {
			#![allow(clippy::semicolon_outside_block)]
			#![allow(clippy::semicolon_if_nothing_returned)]

			#( #clones )*
			#expr
			#trailing_semi
		}};
	};

	// Find the expression to replace
	match &mut input {
		Input::Stmt { stmt } => match stmt {
			syn::Stmt::Local(local) => match &mut local.init {
				// Note: this expression is an initializer, so it never needs the trailing semi
				Some(init) => wrap_expr(&mut init.expr, None),
				None => self::cannot_attach("uninitialized let binding"),
			},
			syn::Stmt::Item(_) => self::cannot_attach("item"),

			// Statement expressions also just carry their previous trailing semicolon
			syn::Stmt::Expr(expr, trailing_semi) => wrap_expr(expr, *trailing_semi),
			syn::Stmt::Macro(_) => self::cannot_attach("macro call"),
		},

		// Normal expressions are the only place we need the user to tell us whether to use a semi or not
		Input::Expr { expr } => wrap_expr(expr, attrs.semi),
	};

	// Then output it.
	let output = match input {
		Input::Stmt { stmt } => stmt.to_token_stream(),
		Input::Expr { expr } => expr.to_token_stream(),
	};

	TokenStream::from(output)
}


/// Input
enum Input {
	Stmt { stmt: syn::Stmt },
	Expr { expr: syn::Expr },
}

impl syn::parse::Parse for Input {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		// Try to parse ourselves as a statement first
		// TODO: The documentation warns against this specifically, but how can we do better?
		let is_stmt = input.fork().parse::<syn::Stmt>().is_ok();
		if is_stmt {
			let stmt = input.parse::<syn::Stmt>()?;
			return Ok(Self::Stmt { stmt });
		}

		// Otherwise, parse an expression
		let expr = input.parse::<syn::Expr>()?;

		// Allow trailing commas so we can parse function arguments
		let _trailing_comma = input.parse::<Option<syn::Token![,]>>()?;

		Ok(Self::Expr { expr })
	}
}

/// Attributes
struct Attrs {
	/// Whether to semi a semicolon at the end
	semi: Option<syn::Token![;]>,

	// All of the clones
	clones: Vec<Attr>,
}

impl syn::parse::Parse for Attrs {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let clones = Punctuated::<_, syn::Token![,]>::parse_terminated_with(input, |input| {
			let attr = input.parse::<Attr>()?;
			let semi = input.parse::<Option<syn::Token![;]>>()?;

			Ok((attr, semi))
		})?;

		let mut semi = None::<syn::Token![;]>;
		let clones = clones
			.into_iter()
			.map(|(attr, new_semi)| {
				if let Some(new_semi) = new_semi {
					match semi {
						Some(old_semi) =>
							return Err(syn::Error::new(
								old_semi.span,
								"Unexpected `;`, only allowed to be trailing",
							)),
						None => semi = Some(new_semi),
					}
				}

				Ok(attr)
			})
			.collect::<Result<_, _>>()?;

		Ok(Self { semi, clones })
	}
}

/// Attribute
struct Attr {
	ident: syn::Ident,
	expr:  Option<syn::Expr>,
}

impl syn::parse::Parse for Attr {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let ident = input.parse()?;
		let expr = match input.parse::<Option<syn::Token![=]>>()? {
			Some(_eq) => {
				let expr = input.parse::<syn::Expr>()?;
				Some(expr)
			},
			None => None,
		};

		Ok(Self { ident, expr })
	}
}

/// Converts an identifier to an expression
fn ident_to_expr(ident: syn::Ident) -> syn::Expr {
	syn::Expr::Path(syn::ExprPath {
		attrs: Vec::new(),
		qself: None,
		path:  syn::Path {
			leading_colon: None,
			segments:      iter::once(syn::PathSegment {
				ident,
				arguments: syn::PathArguments::None,
			})
			.collect(),
		},
	})
}

// Panics with `cannot attach #[cloned] to <kind>`
fn cannot_attach(kind: &str) -> ! {
	panic!("Cannot attach `#[cloned]` to {kind}");
}
