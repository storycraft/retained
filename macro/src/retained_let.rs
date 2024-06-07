use proc_macro2::Span;
use quote::{format_ident, quote_spanned};
use syn::{
    parse::{Parse, ParseStream},
    parse_quote,
    visit::Visit,
    visit_mut::VisitMut,
    AttrStyle, Attribute, Block, Expr, Ident, Index, Local, LocalInit, Meta, Pat, PatIdent,
    PatType, Stmt, Type, TypeTuple,
};

use crate::state::StateField;

pub enum Init {
    Lazy,
    Item,
}

impl Parse for Init {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.step(|cursor| {
            if let Some((ident, rest)) = cursor.ident() {
                if ident == "lazy" {
                    return Ok((Self::Lazy, rest));
                } else if ident == "item" {
                    return Ok((Self::Item, rest));
                }
            }

            Err(cursor.error("expected `lazy or item`"))
        })
    }
}

pub struct RetainedLetAttr {
    pub init: Init,
}

impl RetainedLetAttr {
    pub fn try_from_attr(attr: &Attribute) -> Option<syn::Result<Self>> {
        if !matches!(attr.style, AttrStyle::Outer) || !attr.meta.path().is_ident("retained") {
            return None;
        }

        let Meta::List(ref list) = attr.meta else {
            return Some(Ok(Self { init: Init::Lazy }));
        };

        Some(match list.parse_args::<Init>() {
            Ok(init) => Ok(Self { init }),
            Err(err) => Err(err),
        })
    }
}

pub struct RetainedLetStmt {
    pub attr: RetainedLetAttr,
    pub ty: Type,
    pub init: LocalInit,
}

impl RetainedLetStmt {
    pub fn try_from_local(i: &Local) -> Option<syn::Result<Self>> {
        let attr = i
            .attrs
            .iter()
            .rev()
            .filter_map(RetainedLetAttr::try_from_attr)
            .next()?;

        Some(match attr {
            Ok(attr) => Self::try_from_local_inner(i, attr),
            Err(err) => Err(err),
        })
    }

    fn try_from_local_inner(i: &Local, attr: RetainedLetAttr) -> syn::Result<Self> {
        let LocalTyVisitor { ty: Some(ty) } = ({
            let mut visitor = LocalTyVisitor::default();
            visitor.visit_local(i);

            visitor
        }) else {
            return Err(syn::Error::new_spanned(i, "missing type for retained let"));
        };

        let init = match i.init {
            Some(ref init) => init.clone(),
            None => {
                return Err(syn::Error::new_spanned(
                    i,
                    "missing initializer in retained let",
                ))
            }
        };

        Ok(Self { ty, attr, init })
    }
}

#[derive(Default)]
struct LocalTyVisitor {
    pub ty: Option<Type>,
}

impl Visit<'_> for LocalTyVisitor {
    fn visit_pat_type(&mut self, i: &PatType) {
        self.visit_pat(&i.pat);

        self.ty = Some(Type::clone(&i.ty));
    }
}

pub struct RetainedLetExpander<'a> {
    state: Ident,
    block_state: Ident,
    depth: usize,
    fields: &'a mut Vec<StateField>,
    stack: Vec<Type>,
}

impl<'a> RetainedLetExpander<'a> {
    pub fn expand(state: &Ident, depth: usize, fields: &'a mut Vec<StateField>, block: &mut Block) {
        let block_state = format_ident!("{}{}", state, depth, span = Span::mixed_site());

        let mut this = Self {
            state: state.clone(),
            block_state,
            depth,
            fields,
            stack: Vec::new(),
        };

        for stmt in &mut block.stmts {
            this.visit_stmt_mut(stmt);

            *stmt = match stmt {
                Stmt::Local(ref mut local) => match RetainedLetStmt::try_from_local(local) {
                    Some(Ok(retained_let)) => this.low(local, retained_let),

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

        let index = Index::from(this.fields.len());
        this.fields.push(StateField {
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

        let block_state = &this.block_state;
        block.stmts.insert(
            0,
            Stmt::Expr(
                Expr::Verbatim(quote_spanned! { Span::mixed_site() =>
                    let #block_state = &mut #state. #index;
                }),
                Some(Default::default()),
            ),
        );
    }

    fn low(
        &mut self,
        local: &mut Local,
        RetainedLetStmt { attr, init, ty }: RetainedLetStmt,
    ) -> Stmt {
        match attr.init {
            Init::Lazy => self.low_lazy(local, init, ty),
            Init::Item => match self.low_item(local, init, ty) {
                Ok(stmt) => stmt,

                Err(err) => Stmt::Expr(
                    Expr::Verbatim(err.to_compile_error()),
                    Some(Default::default()),
                ),
            },
        }
    }

    fn low_item(&mut self, local: &mut Local, init: LocalInit, ty: Type) -> syn::Result<Stmt> {
        if init.diverge.is_some() {
            return Err(syn::Error::new_spanned(
                local,
                "item retained let cannot diverge",
            ));
        }
        let index = Index::from(self.fields.len());
        self.fields.push(StateField {
            ty,
            init: *init.expr,
        });

        let pat = &local.pat;

        let state = &self.state;
        Ok(Stmt::Expr(
            Expr::Verbatim(quote_spanned!(Span::mixed_site() =>
                let #pat = #state . #index;
            )),
            Some(Default::default()),
        ))
    }

    fn low_lazy(&mut self, local: &mut Local, init: LocalInit, ty: Type) -> Stmt {
        self.stack.push(ty.clone());

        let init_ident = Ident::new("__init", Span::mixed_site());
        let pat = &local.pat;
        let init_var = Local {
            attrs: vec![],
            let_token: Default::default(),
            pat: Pat::Ident(PatIdent {
                attrs: vec![],
                by_ref: None,
                mutability: None,
                ident: init_ident.clone(),
                subpat: None,
            }),
            init: Some(init),
            semi_token: Default::default(),
        };

        let block_state = &self.block_state;
        Stmt::Expr(
            Expr::Verbatim(quote_spanned!(Span::mixed_site() =>
                let (ref mut __tmp, ref mut #block_state) = {
                    if #block_state.is_none() {
                        * #block_state = ::core::option::Option::Some(({
                            #init_var
                            #init_ident
                        }, Default::default()));
                    }

                    #block_state .as_mut().unwrap()
                };

                let #pat = *__tmp;
            )),
            Some(Default::default()),
        )
    }
}

impl VisitMut for RetainedLetExpander<'_> {
    fn visit_block_mut(&mut self, i: &mut syn::Block) {
        if i.stmts.is_empty() {
            return;
        }

        RetainedLetExpander::expand(&self.state, self.depth + 1, self.fields, i);
    }
}
