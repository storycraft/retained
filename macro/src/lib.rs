mod retained_let;
mod state;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote_spanned;
use retained_let::RetainedLetExpander;
use state::{State, StateArg, StateDecl};
use syn::{parse_macro_input, parse_quote, Ident, ItemFn};

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
        decl: &state.decl,
    };

    RetainedLetExpander::expand(&state_arg.name, 0, &mut state.fields, &mut f.block);

    f.sig.inputs.push(parse_quote!(#state_arg));

    TokenStream::from(quote_spanned! { Span::mixed_site() =>
        #state
        #f
    })
}
