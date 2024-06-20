use proc_macro2::Span;
use quote::quote_spanned;
use syn::{visit::Visit, Expr, Ident, Index, Local, Pat, PatIdent, Stmt, Type};

use crate::state::{State, StateField, StateProvided};

use super::LocalTyVisitor;

pub struct ProvidedLetStmt {
    pub name: Ident,
    pub pat: Pat,
    pub ty: Type,
}

impl ProvidedLetStmt {
    pub fn try_from(local: &Local) -> syn::Result<Self> {
        let Some(ty) = LocalTyVisitor::find(local) else {
            return Err(syn::Error::new_spanned(
                local,
                "missing type for retained let",
            ));
        };

        if local.init.is_some() {
            return Err(syn::Error::new_spanned(
                local,
                "provided retained let cannot have initializer",
            ));
        }

        let Some(name) = LocalNameVisitor::find(local) else {
            return Err(syn::Error::new_spanned(
                local,
                "provided retained let must have binding initializer",
            ));
        };

        Ok(Self {
            name,
            pat: local.pat.clone(),
            ty,
        })
    }

    pub fn low(self, state_arg: &Ident, state: &mut State) -> Stmt {
        let Self { name, pat, ty } = self;
        let index = Index::from(state.fields.len());
        state.new_args.push(StateProvided {
            name: name.clone(),
            ty: ty.clone(),
        });
        state.fields.push(StateField {
            ty,
            init: Expr::Verbatim(quote_spanned!(Span::mixed_site() => {
                #name
            })),
        });

        Stmt::Expr(
            Expr::Verbatim(quote_spanned!(Span::mixed_site() =>
                let #pat = #state_arg . #index;
            )),
            Some(Default::default()),
        )
    }
}

struct LocalNameVisitor {
    pub ident: Option<Ident>,
}

impl LocalNameVisitor {
    pub fn find(local: &Local) -> Option<Ident> {
        let mut this = Self { ident: None };
        this.visit_local(local);

        this.ident
    }
}

impl Visit<'_> for LocalNameVisitor {
    fn visit_pat_ident(&mut self, i: &PatIdent) {
        self.ident = Some(i.ident.clone());
    }
}
