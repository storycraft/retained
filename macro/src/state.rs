use proc_macro2::Span;
use quote::{quote_spanned, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    Expr, Generics, Ident, Type, Visibility,
};

#[derive(Clone)]
pub struct StateDecl {
    pub name: Ident,
    pub generics: Generics,
}

impl Parse for StateDecl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            name: input.parse()?,
            generics: input.parse()?,
        })
    }
}

pub struct StateField {
    pub ty: Type,
    pub init: Expr,
}

pub struct State {
    pub vis: Visibility,
    pub decl: StateDecl,
    pub fields: Vec<StateField>,
}

impl ToTokens for State {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            vis,
            decl: StateDecl { name, generics },
            fields,
        } = self;

        let params = &generics.params;

        let inner_name = quote::format_ident!("__{}", name, span = Span::mixed_site());

        let field_ty_iter = fields.iter().map(|field| &field.ty);
        let field_init_iter = fields.iter().map(|field| &field.init);

        *tokens = quote_spanned!(Span::mixed_site() =>
            struct #inner_name #generics(#(#field_ty_iter),*);

            #[repr(transparent)]
            #[non_exhaustive]
            #vis struct #name #generics(
                #inner_name #generics,
            );

            const _: () = {
                impl<#params> ::core::default::Default for #name #generics {
                    fn default() -> Self {
                        Self::new()
                    }
                }

                impl<#params> ::core::fmt::Debug for #name #generics {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                        f.debug_struct(::core::stringify!(#name)).finish_non_exhaustive()
                    }
                }

                impl<#params> #name #generics {
                    pub fn new() -> Self {
                        Self(#inner_name (#(#field_init_iter),*))
                    }
                }
            };
        );
    }
}

pub struct StateArg<'a> {
    pub name: Ident,
    pub decl: &'a StateDecl,
}

impl ToTokens for StateArg<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let StateArg {
            name,
            decl:
                StateDecl {
                    name: state_ty,
                    generics,
                },
        } = self;

        *tokens = quote_spanned! { Span::mixed_site() =>
            #state_ty (#name) : &mut #state_ty #generics
        };
    }
}
