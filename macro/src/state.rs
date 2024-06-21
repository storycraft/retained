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
            generics: {
                let mut generics: Generics = input.parse()?;
                generics.where_clause = input.parse()?;
                generics
            },
        })
    }
}

pub struct StateField {
    pub ty: Type,
    pub init: Expr,
}

pub struct StateProvided {
    pub name: Ident,
    pub ty: Type,
}

impl ToTokens for StateProvided {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self { name, ty } = self;

        *tokens = quote_spanned!(Span::mixed_site() =>
            #name : #ty
        );
    }
}

pub struct State {
    pub vis: Visibility,
    pub decl: StateDecl,
    pub fields: Vec<StateField>,
    pub new_args: Vec<StateProvided>,
}

impl ToTokens for State {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            vis,
            decl: StateDecl { name, generics },
            new_args,
            fields,
        } = self;

        let (impl_gen, ty_gen, where_gen) = generics.split_for_impl();

        let inner_name = quote::format_ident!("__{}", name, span = Span::mixed_site());

        let field_ty_iter = fields.iter().map(|field| &field.ty);
        let field_init_iter = fields.iter().map(|field| &field.init);

        *tokens = quote_spanned!(Span::mixed_site() =>
            struct #inner_name #ty_gen (#(#field_ty_iter),*) #where_gen;

            #[repr(transparent)]
            #[non_exhaustive]
            #vis struct #name #ty_gen (
                #inner_name #ty_gen,
            ) #where_gen;

            const _: () = {
                impl #impl_gen ::core::fmt::Debug for #name #ty_gen #where_gen {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                        f.debug_struct(::core::stringify!(#name)).finish_non_exhaustive()
                    }
                }

                impl #impl_gen #name #ty_gen #where_gen {
                    pub fn new(#(#new_args),* ) -> Self {
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
