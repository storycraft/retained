mod state;
mod retained_let;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote_spanned, ToTokens};
use retained_let::RetainedLetExpander;
use state::{State, StateArg, StateDecl};
use syn::{
    parse_macro_input,
    visit_mut::VisitMut, FnArg, Ident, ItemFn,
};

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
        decl: state.decl.clone(),
    };

    RetainedLetExpander {
        state_arg: &state_arg,
        fields: &mut state.fields,
    }
    .visit_block_mut(&mut f.block);

    f.sig.inputs.push({
        let s = TokenStream::from(state_arg.to_token_stream());
        parse_macro_input!(s as FnArg)
    });

    TokenStream::from(quote_spanned! { Span::mixed_site() =>
        #state
        #f
    })
}
