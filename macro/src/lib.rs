mod retained_let;
mod state;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote_spanned;
use retained_let::RetainedLetExpander;
use state::{State, StateArg, StateDecl};
use syn::{parse_macro_input, parse_quote, Ident, ItemFn};

/// Create external storage for tagged local variables and bind to original let statement.
///
/// This macro cannot be used inside impl block.
/// ```compile_fail
/// impl Struct {
///     #[retained(State)]
///     pub fn method(&self) {
///         ..
///     }
/// }
/// ```
///
/// ## Usage
/// The `retained` macro can only be used on top of bare function.
/// It takes identifier and optionally generics paremeters to build state struct declaration.
/// The macro will make a storage for local variables tagged with `#[retained]`.
///
/// Tagged let statement requires type and initializer like it is `static` or `const`.
/// Corresponding fields in state struct are initialized on first access and bound to original let statment.
///
/// The following does not compile as it will move state's field to local variable.
/// ```compile_fail
/// #[retained(State)]
/// fn my_fn() {
///     #[retained]
///     let retained_string: String = String::new("");
/// }
/// ```
///
/// To make this work, use ref pattern instead.
/// ```no_run
/// #[retained(State)]
/// fn my_fn() {
///     #[retained]
///     let ref retained_string: String = String::new("");
///     // Mutable access
///     // let ref mut retained_string: String = String::new("");
/// }
/// ```
#[proc_macro_attribute]
pub fn retained(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut f = parse_macro_input!(item as ItemFn);

    let mut state = State {
        vis: f.vis.clone(),
        decl: parse_macro_input!(attr as StateDecl),
        fields: Vec::new(),
        new_args: Vec::new(),
    };

    let name = Ident::new("__inner", Span::mixed_site());
    RetainedLetExpander::expand(name.clone(), 0, &mut state, &mut f.block);

    let state_arg = StateArg {
        name,
        decl: &state.decl,
    };
    f.sig.inputs.push(parse_quote!(#state_arg));

    TokenStream::from(quote_spanned! { Span::mixed_site() =>
        #state
        #f
    })
}
