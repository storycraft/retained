use proc_macro2::Span;
use quote::{format_ident, quote_spanned};
use syn::{
    parse_quote, visit::Visit, visit_mut::VisitMut, AttrStyle, Block, Expr,
    Ident, Index, Local, LocalInit, Pat, PatIdent, PatType, Stmt, Type, TypeTuple,
};

use crate::state::StateField;

pub struct RetainedLetStmt {
    pub pat: Pat,
    pub ty: Type,
    pub init: LocalInit,
}

impl RetainedLetStmt {
    pub fn try_from_local(i: &Local) -> syn::Result<Self> {
        let LocalTyVisitor { ty: Some(ty) } = ({
            let mut visitor = LocalTyVisitor::default();
            visitor.visit_local(i);

            visitor
        }) else {
            return Err(syn::Error::new_spanned(i, "missing type for retained let"));
        };

        let Some(init) = i.init.as_ref() else {
            return Err(syn::Error::new_spanned(
                i,
                "missing initializer in retained let",
            ));
        };

        Ok(Self {
            pat: i.pat.clone(),
            ty,
            init: init.clone(),
        })
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
    pub fn expand(
        state: &Ident,
        depth: usize,
        fields: &'a mut Vec<StateField>,
        block: &mut Block,
    ) {
        let block_state = format_ident!(
            "{}{}",
            state,
            depth,
            span = Span::mixed_site()
        );

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
                Stmt::Local(ref mut local)
                    if local.attrs.iter().any(|attr| {
                        matches!(attr.style, AttrStyle::Outer)
                            && attr.meta.path().is_ident("retained")
                    }) =>
                {
                    this.low(local)
                }

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

    fn low(&mut self, local: &mut Local) -> Stmt {
        let RetainedLetStmt { pat, ty, init } = match RetainedLetStmt::try_from_local(local) {
            Ok(res) => res,

            Err(err) => {
                return Stmt::Expr(
                    Expr::Verbatim(err.to_compile_error()),
                    Some(Default::default()),
                );
            }
        };

        self.stack.push(ty.clone());

        let init_ident = Ident::new("__init", Span::mixed_site());

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
