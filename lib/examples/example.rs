use std::fmt::Debug;

use retained::retained;

#[retained(State<T>)]
pub fn asdf<T: Debug>(input: T, input2: &str) {
    #[retained]
    let ref a: T = input;
    dbg!(a);

    #[retained]
    let ref mut nested: State2 = State2::new();
    asdf2(input2, nested);
}

#[retained(State2)]
pub fn asdf2(text: &str) {
    #[retained]
    let ref text: String = text.to_string();

    println!("text: {text}");
}

fn main() {
    let state = &mut State::new();

    asdf(&123, "Hello world", state);
    asdf(&456, "Hello world2", state);
}
