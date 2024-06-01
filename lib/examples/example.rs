use std::fmt::Debug;

use retained::retained;

#[retained(State<T>)]
pub fn asdf<T: Debug>(input: T, input2: &str) {
    #[retained]
    let ref a: T = input;

    #[retained]
    let ref mut nested: State2 = State2::new();
    asdf2(input2, nested);

    dbg!(a);
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
    asdf(&456, "", state);
    asdf(&789, "", state);
    asdf(&012, "world", state);
    asdf(&345, "Hello", state);
}
