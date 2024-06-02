use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    visit::Visit,
    visit_mut::VisitMut,
    AttrStyle, Expr, FnArg, Generics, Ident, Index, ItemFn, Local, LocalInit, Pat, PatIdent,
    PatType, Stmt, Token, Type, Visibility,
};

#[derive(Clone)]
struct StateDecl {
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

struct RetainedLetStmt {
    pub sig: (Option<Token![ref]>, Option<Token![mut]>),
    pub name: Ident,
    pub ty: Type,
    pub init: LocalInit,
}

impl RetainedLetStmt {
    pub fn try_from_local(i: &Local) -> syn::Result<Self> {
        #[derive(Default)]
        struct LocalVisitor {
            pub ident: Option<PatIdent>,
            pub ty: Option<Type>,
        }

        impl Visit<'_> for LocalVisitor {
            fn visit_pat_ident(&mut self, i: &PatIdent) {
                self.ident = Some(i.clone());
            }

            fn visit_pat_type(&mut self, i: &PatType) {
                self.visit_pat(&i.pat);

                self.ty = Some(Type::clone(&i.ty));
            }
        }

        let LocalVisitor {
            ident:
                Some(PatIdent {
                    by_ref,
                    mutability,
                    ident,
                    ..
                }),
            ty: Some(ty),
        } = ({
            let mut visitor = LocalVisitor::default();
            visitor.visit_local(i);

            visitor
        })
        else {
            return Err(syn::Error::new_spanned(i, "invalid retained let"));
        };

        let init = i
            .init
            .as_ref()
            .ok_or_else(|| syn::Error::new_spanned(i, "missing initializer in retained let"))?;

        Ok(Self {
            sig: (by_ref, mutability),
            name: ident.clone(),
            ty,
            init: init.clone(),
        })
    }
}

struct State {
    vis: Visibility,
    decl: StateDecl,
    fields: Vec<StateField>,
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

        let field_tys = fields.iter().map(|field| &field.ty);
        let field_default = quote!(::core::option::Option::None);
        let field_default = [&field_default].into_iter().cycle().take(fields.len());

        *tokens = quote! {
            #vis struct #inner_name #generics(#(::core::option::Option<#field_tys>),*);

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
        };
    }
}

struct StateField {
    pub ty: Type,
}

impl ToTokens for StateField {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.ty.to_tokens(tokens)
    }
}

struct StateArg {
    pub name: Ident,
    pub colon: Token![:],
    pub decl: StateDecl,
}

impl ToTokens for StateArg {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let StateArg {
            name,
            colon,
            decl: StateDecl {
                name: ty_name,
                generics,
            },
        } = self;

        *tokens = quote! {
            #name #colon &mut #ty_name #generics
        };
    }
}

struct RetainedLetVisitor<'a, 'b> {
    pub state_arg: &'a StateArg,
    pub fields: &'b mut Vec<StateField>,
}

impl VisitMut for RetainedLetVisitor<'_, '_> {
    fn visit_block_mut(&mut self, i: &mut syn::Block) {
        for stmt in &mut i.stmts {
            self.visit_stmt_mut(stmt);

            if let Stmt::Local(local) = stmt {
                if !local.attrs.iter().any(|attr| {
                    matches!(attr.style, AttrStyle::Outer) && attr.meta.path().is_ident("retained")
                }) {
                    continue;
                }

                let RetainedLetStmt {
                    sig: (ref_token, mut_token),
                    name,
                    ty,
                    init,
                } = match RetainedLetStmt::try_from_local(local) {
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
                self.fields.push(StateField { ty: ty.clone() });

                let init_var_name = Ident::new("__init", name.span());

                let init_var = Local {
                    attrs: vec![],
                    let_token: Default::default(),
                    pat: Pat::Ident(PatIdent {
                        attrs: vec![],
                        by_ref: None,
                        mutability: None,
                        ident: init_var_name.clone(),
                        subpat: None,
                    }),
                    init: Some(init),
                    semi_token: Default::default(),
                };

                *stmt = Stmt::Expr(
                    Expr::Verbatim(quote! {
                        let __tmp = ::retained::__private::Ptr::new(
                            ::core::ptr::addr_of_mut!(#state_name .0. #index)
                        ).cast::<::core::option::Option<#ty>>();

                        let #ref_token #mut_token #name = *{
                            let __tmp = unsafe { __tmp.as_mut() };

                            if __tmp.is_none() {
                                *__tmp = ::core::option::Option::Some({
                                    #init_var
                                    #init_var_name
                                });
                            }

                            __tmp
                        }.as_mut().unwrap();
                    }),
                    Some(Default::default()),
                );
            }
        }
    }

    // ignore items
    fn visit_item_mut(&mut self, _: &mut syn::Item) {}
}

#[proc_macro_attribute]
pub fn retained(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut f = parse_macro_input!(item as ItemFn);

    let mut state = State {
        vis: f.vis.clone(),
        decl: parse_macro_input!(attr as StateDecl),
        fields: Vec::new(),
    };

    let state_arg = StateArg {
        name: Ident::new("__inner", Span::mixed_site()),
        colon: Default::default(),
        decl: state.decl.clone(),
    };

    RetainedLetVisitor {
        state_arg: &state_arg,
        fields: &mut state.fields,
    }
    .visit_block_mut(&mut f.block);

    f.sig.inputs.push({
        let s = TokenStream::from(state_arg.to_token_stream());
        parse_macro_input!(s as FnArg)
    });

    TokenStream::from(quote! {
        #state
        #f
    })
}
