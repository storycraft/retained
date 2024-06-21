mod default;
mod inplace;

pub use default::DefaultLetStmt;
pub use inplace::InplaceLetStmt;

use proc_macro2::Span;
use quote::{format_ident, quote_spanned};
use syn::{
    parse::{Parse, ParseStream},
    parse_quote,
    visit::Visit,
    visit_mut::VisitMut,
    AttrStyle, Attribute, Block, Expr, Ident, Index, Local, LocalInit, Meta, PatType, Stmt, Type,
    TypeTuple,
};

use crate::state::{State, StateField};

enum InitMode {
    Inplace,
    Default,
}

impl Parse for InitMode {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.step(|cursor| {
            if let Some((ident, rest)) = cursor.ident() {
                if ident == "inplace" {
                    return Ok((Self::Inplace, rest));
                } else if ident == "default" {
                    return Ok((Self::Default, rest));
                }
            }

            Err(cursor.error("expected `inplace` or `default`"))
        })
    }
}

struct Attr(InitMode);

impl Attr {
    pub fn try_from_attr(attr: &Attribute) -> Option<syn::Result<Self>> {
        if !matches!(attr.style, AttrStyle::Outer) || !attr.meta.path().is_ident("retained") {
            return None;
        }

        let Meta::List(ref list) = attr.meta else {
            return Some(Ok(Self(InitMode::Inplace)));
        };

        Some(match list.parse_args::<InitMode>() {
            Ok(init) => Ok(Self(init)),
            Err(err) => Err(err),
        })
    }
}

pub enum RetainedLetStmt {
    Inplace(InplaceLetStmt),
    Default(DefaultLetStmt),
}

impl RetainedLetStmt {
    pub fn try_from_local(i: &Local) -> Option<syn::Result<Self>> {
        let attr = i
            .attrs
            .iter()
            .rev()
            .filter_map(Attr::try_from_attr)
            .next()?;

        Some(match attr {
            Ok(attr) => Self::try_from_local_inner(i, attr),
            Err(err) => Err(err),
        })
    }

    fn try_from_local_inner(i: &Local, Attr(init): Attr) -> syn::Result<Self> {
        Ok(match init {
            InitMode::Inplace => Self::Inplace(InplaceLetStmt::try_from(i)?),
            InitMode::Default => Self::Default(DefaultLetStmt::try_from(i)?),
        })
    }
}

pub struct RetainedLetExpander<'a> {
    state_arg: Ident,
    block_state: Ident,
    depth: usize,
    state: &'a mut State,
    stack: Vec<Type>,
}

impl<'a, 'b> RetainedLetExpander<'a> {
    pub fn expand(state_arg: Ident, depth: usize, state: &'a mut State, block: &mut Block) {
        let block_state = format_ident!("{}{}", state_arg, depth, span = Span::mixed_site());

        let mut this = Self {
            state_arg,
            block_state,
            depth,
            state,
            stack: Vec::new(),
        };

        for stmt in &mut block.stmts {
            this.visit_stmt_mut(stmt);

            *stmt = match stmt {
                Stmt::Local(ref mut local) => match RetainedLetStmt::try_from_local(local) {
                    Some(Ok(retained_let)) => this.low(retained_let),

                    Some(Err(err)) => Stmt::Expr(
                        Expr::Verbatim(err.to_compile_error()),
                        Some(Default::default()),
                    ),

                    _ => continue,
                },
                _ => continue,
            };
        }

        if this.stack.is_empty() {
            return;
        }

        let index = Index::from(this.state.fields.len());
        this.state.fields.push(StateField {
            ty: {
                let mut state_ty = Type::Tuple(TypeTuple {
                    paren_token: Default::default(),
                    elems: Default::default(),
                });

                for ty in this.stack.into_iter().rev() {
                    state_ty = parse_quote!(
                        ::core::option::Option<(#ty , #state_ty)>
                    );
                }

                state_ty
            },
            init: parse_quote!(::core::option::Option::None),
        });

        let state_arg = &this.state_arg;
        let block_state = &this.block_state;
        block.stmts.insert(
            0,
            Stmt::Expr(
                Expr::Verbatim(quote_spanned! { Span::mixed_site() =>
                    let #block_state = &mut #state_arg. #index;
                }),
                Some(Default::default()),
            ),
        );
    }

    fn low(&mut self, retaind_let: RetainedLetStmt) -> Stmt {
        match retaind_let {
            RetainedLetStmt::Inplace(inplace) => inplace.low(&self.block_state, &mut self.stack),
            RetainedLetStmt::Default(default) => default.low(&self.state_arg, self.state),
        }
    }
}

impl VisitMut for RetainedLetExpander<'_> {
    fn visit_block_mut(&mut self, i: &mut syn::Block) {
        if i.stmts.is_empty() {
            return;
        }

        RetainedLetExpander::expand(self.state_arg.clone(), self.depth + 1, self.state, i);
    }
}

struct LocalTyVisitor {
    pub ty: Option<Type>,
}

impl LocalTyVisitor {
    pub fn find(local: &Local) -> Option<Type> {
        let mut this = Self { ty: None };
        this.visit_local(local);

        this.ty
    }
}

impl Visit<'_> for LocalTyVisitor {
    fn visit_pat_type(&mut self, i: &PatType) {
        self.visit_pat(&i.pat);

        self.ty = Some(Type::clone(&i.ty));
    }
}

fn extract_init(local: &Local) -> syn::Result<Expr> {
    match local.init {
        Some(LocalInit {
            diverge: Some((_, ref diverge)),
            ..
        }) => Err(syn::Error::new_spanned(
            diverge,
            "retained let cannot diverge",
        )),

        Some(LocalInit { ref expr, .. }) => Ok(Expr::clone(expr)),

        None => Err(syn::Error::new_spanned(
            local,
            "missing initializer in retained let",
        )),
    }
}
