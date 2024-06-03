use proc_macro2::Span;
use quote::quote_spanned;
use syn::{
    visit::Visit, visit_mut::VisitMut, AttrStyle, Expr, Ident, Index, Local, LocalInit, Pat,
    PatIdent, PatType, Stmt, Type,
};

use crate::state::StateArg;

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

pub struct RetainedLetExpander<'a, 'b> {
    pub state_arg: &'a StateArg,
    pub fields: &'b mut Vec<Type>,
}

impl VisitMut for RetainedLetExpander<'_, '_> {
    fn visit_block_mut(&mut self, i: &mut syn::Block) {
        for stmt in &mut i.stmts {
            self.visit_stmt_mut(stmt);

            if let Stmt::Local(local) = stmt {
                if !local.attrs.iter().any(|attr| {
                    matches!(attr.style, AttrStyle::Outer) && attr.meta.path().is_ident("retained")
                }) {
                    continue;
                }

                let RetainedLetStmt { pat, ty, init } = match RetainedLetStmt::try_from_local(local)
                {
                    Ok(res) => res,

                    Err(err) => {
                        *stmt = Stmt::Expr(
                            Expr::Verbatim(err.to_compile_error()),
                            Some(Default::default()),
                        );
                        continue;
                    }
                };

                let StateArg {
                    name: ref state_name,
                    ..
                } = self.state_arg;

                let index = Index::from(self.fields.len());
                self.fields.push(ty.clone());

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

                *stmt = Stmt::Expr(
                    Expr::Verbatim(quote_spanned!(Span::mixed_site() =>
                        let __tmp = &mut #state_name .0. #index;

                        let #pat = *{
                            if __tmp.is_none() {
                                *__tmp = ::core::option::Option::Some({
                                    #init_var
                                    #init_ident
                                });
                            }

                            __tmp
                        }.as_mut().unwrap();
                    )),
                    Some(Default::default()),
                );
            }
        }
    }

    // ignore inner items
    fn visit_item_mut(&mut self, _: &mut syn::Item) {}
}
