//! Macros for `zutil-cloned`

// Features
#![feature(if_let_guard, try_blocks)]

// Imports
use {
	core::iter,
	proc_macro::TokenStream,
	quote::ToTokens,
	syn::{punctuated::Punctuated, spanned::Spanned, Token},
};


#[proc_macro_attribute]
pub fn cloned(attr: TokenStream, input: TokenStream) -> TokenStream {
	let attrs = syn::parse_macro_input!(attr as Attrs);
	let mut input = syn::parse_macro_input!(input as Input);

	let clones = attrs
		.0
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
			#( #clones )*
			#expr
			#trailing_semi
		}};
	};

	// Find the expression to replace
	match &mut input {
		Input::Stmt(stmt) => match stmt {
			syn::Stmt::Local(local) => match &mut local.init {
				Some(init) => wrap_expr(&mut init.expr, None),
				None => self::cannot_attach("uninitialized let binding"),
			},
			syn::Stmt::Item(_) => self::cannot_attach("item"),
			syn::Stmt::Expr(expr, trailing_semi) => wrap_expr(expr, *trailing_semi),
			syn::Stmt::Macro(_) => self::cannot_attach("macro call"),
		},
		// On expressions, use a `;`, unless we have a trailing comma.
		Input::Expr(expr, trailing_comma) => wrap_expr(expr, match trailing_comma {
			Some(_) => None,
			None => Some(syn::parse_quote!(;)),
		}),
	};

	// Then output it.
	let output = match input {
		Input::Stmt(stmt) => stmt.to_token_stream(),
		Input::Expr(expr, _) => expr.to_token_stream(),
	};

	TokenStream::from(output)
}


/// Input
#[derive(Debug)]
enum Input {
	Stmt(syn::Stmt),
	Expr(syn::Expr, Option<Token![,]>),
}

impl syn::parse::Parse for Input {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		// Try to parse ourselves as a statement first
		// TODO: The documentation warns against this specifically, but how can we do better?
		let is_stmt = input.fork().parse::<syn::Stmt>().is_ok();
		if is_stmt {
			let stmt = input.parse::<syn::Stmt>()?;
			return Ok(Self::Stmt(stmt));
		}

		// Otherwise, parse an expression
		let expr = input.parse::<syn::Expr>()?;

		// Allow trailing commas so we can parse function arguments
		let trailing_comma = input.parse::<Option<Token![,]>>()?;

		Ok(Self::Expr(expr, trailing_comma))
	}
}

/// Attributes
struct Attrs(Punctuated<Attr, Token![,]>);

impl syn::parse::Parse for Attrs {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let attrs = Punctuated::parse_terminated(input)?;
		Ok(Self(attrs))
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
