use proc_macro2::Span;
use quote::quote_spanned;
use syn::{Expr, Ident, Index, Local, Pat, Stmt, Type};

use crate::state::{State, StateField};

use super::{extract_init, LocalTyVisitor};

pub struct DefaultLetStmt {
    pub pat: Pat,
    pub ty: Type,
    pub init: Expr,
}

impl DefaultLetStmt {
    pub fn try_from(local: &Local) -> syn::Result<Self> {
        let Some(ty) = LocalTyVisitor::find(local) else {
            return Err(syn::Error::new_spanned(
                local,
                "missing type for retained let",
            ));
        };
        let init = extract_init(&local)?;

        Ok(Self {
            pat: local.pat.clone(),
            ty,
            init,
        })
    }

    pub fn low(self, state_arg: &Ident, state: &mut State) -> Stmt {
        let index = Index::from(state.fields.len());
        state.fields.push(StateField {
            ty: self.ty,
            init: self.init,
        });

        let pat = &self.pat;
        Stmt::Expr(
            Expr::Verbatim(quote_spanned!(Span::mixed_site() =>
                let #pat = #state_arg . #index;
            )),
            Some(Default::default()),
        )
    }
}
