use std::fmt::Display;

use retained::retained;

#[retained(State<T>)]
pub fn display<T: Display>(input: T, input2: &str) {
    #[retained]
    let ref input: T = input;

    #[retained]
    let ref mut nested: State2 = State2::new();
    display_str(input2, nested);

    println!("input: {input}");
}

#[retained(State2)]
pub fn display_str(text: &str) {
    #[retained]
    let ref text: String = text.to_string();

    println!("text: {text}");
}

fn main() {
    let state = &mut State::new();

    // initialize states and print
    //
    // text: Hello world
    // input: 123
    display(&123, "Hello world", state);

    // different arguments are given,
    // but print same as first since states are already initialized.
    display(&456, "", state);
    display(&789, "", state);
    display(&012, "world", state);
    display(&345, "Hello", state);
}
