use proc_macro2::Span;
use quote::{quote_spanned, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    Generics, Ident, Type, Visibility,
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

pub struct State {
    pub vis: Visibility,
    pub decl: StateDecl,
    pub fields: Vec<Type>,
}

impl ToTokens for State {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            vis,
            decl: StateDecl { name, generics },
            fields,
        } = self;

        let params = &generics.params;

        let inner_name = quote::format_ident!("__{}", name);

        let field_default = quote_spanned!(
            Span::mixed_site() => ::core::option::Option::None
        );
        let field_default = [&field_default].into_iter().cycle().take(fields.len());

        *tokens = quote_spanned!(Span::mixed_site() =>
            #vis struct #inner_name #generics(#(::core::option::Option<#fields>),*);

            #[repr(transparent)]
            #vis struct #name #generics(
                #inner_name #generics,
            );

            const _: () = {
                impl<#params> ::core::default::Default for #name #generics {
                    fn default() -> Self {
                        Self::new()
                    }
                }

                impl<#params> #name #generics {
                    pub const fn new() -> Self {
                        Self(#inner_name (#(#field_default),*))
                    }
                }
            };
        );
    }
}

pub struct StateArg {
    pub name: Ident,
    pub decl: StateDecl,
}

impl ToTokens for StateArg {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let StateArg {
            name,
            decl: StateDecl {
                name: ty_name,
                generics,
            },
        } = self;

        *tokens = quote_spanned! { Span::mixed_site() =>
            #name : &mut #ty_name #generics
        };
    }
}
