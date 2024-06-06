# `retained`
[Documentation](https://docs.rs/retained/latest)

Keep local variables between repeated function calls using simple macro.

This crate is no_std.

## Usage
```rust no_run
use retained::retained;

#[retained(DrawState)]
fn draw() {
    #[retained]
    let ref mut check_box: CheckBox = CheckBox::new(/* checked */ false);
    check_box.draw();
}

fn draw_loop() {
    let mut state = DrawState::new();

    loop {
        draw(&mut state);
    }
}
```
Without `retained`, Checkbox's states would be reset on every `draw` call.
By using `retained`, local variable `check_box` is kept inside `DrawState` struct.
And `draw` function gets additional `&mut DrawState` argument.

## Examples
See `examples` for simple example and egui demo ported using `retained`.

## License
This crate is licensed under MIT OR Apache-2.0