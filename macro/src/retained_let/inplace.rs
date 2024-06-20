use proc_macro2::Span;
use quote::quote_spanned;
use syn::{Expr, Ident, Local, Pat, Stmt, Type};

use super::{extract_init, LocalTyVisitor};

pub struct InplaceLetStmt {
    pub pat: Pat,
    pub ty: Type,
    pub init: Expr,
}

impl InplaceLetStmt {
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

    pub fn low(self, block_state: &Ident, stack: &mut Vec<Type>) -> Stmt {
        let Self { pat, ty, init } = self;
        stack.push(ty);

        Stmt::Expr(
            Expr::Verbatim(quote_spanned!(Span::mixed_site() =>
                let (ref mut __tmp, ref mut #block_state) = {
                    if #block_state.is_none() {
                        * #block_state = ::core::option::Option::Some(({
                            #init
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
