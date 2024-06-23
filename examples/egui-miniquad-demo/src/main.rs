//! Ported from <https://github.com/not-fl3/egui-miniquad/blob/master/examples/demo.rs>

use {egui_miniquad as egui_mq, miniquad as mq, retained::retained};

struct Stage {
    egui_mq: egui_mq::EguiMq,
    state: State,
}

impl Stage {
    fn new() -> Self {
        let mut mq_ctx = mq::window::new_rendering_backend();

        Self {
            egui_mq: egui_mq::EguiMq::new(&mut *mq_ctx),
            state: State::new(mq_ctx),
        }
    }
}

impl mq::EventHandler for Stage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        draw(&mut self.egui_mq, &mut self.state);
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        self.egui_mq.mouse_motion_event(x, y);
    }

    fn mouse_wheel_event(&mut self, dx: f32, dy: f32) {
        self.egui_mq.mouse_wheel_event(dx, dy);
    }

    fn mouse_button_down_event(&mut self, mb: mq::MouseButton, x: f32, y: f32) {
        self.egui_mq.mouse_button_down_event(mb, x, y);
    }

    fn mouse_button_up_event(&mut self, mb: mq::MouseButton, x: f32, y: f32) {
        self.egui_mq.mouse_button_up_event(mb, x, y);
    }

    fn char_event(&mut self, character: char, _keymods: mq::KeyMods, _repeat: bool) {
        self.egui_mq.char_event(character);
    }

    fn key_down_event(&mut self, keycode: mq::KeyCode, keymods: mq::KeyMods, _repeat: bool) {
        self.egui_mq.key_down_event(keycode, keymods);
    }

    fn key_up_event(&mut self, keycode: mq::KeyCode, keymods: mq::KeyMods) {
        self.egui_mq.key_up_event(keycode, keymods);
    }
}

#[retained(State(mq_ctx: Box<dyn mq::RenderingBackend>))]
fn draw(egui_mq: &mut egui_mq::EguiMq) {
    #[retained(default)]
    let ref mut mq_ctx: Box<dyn mq::RenderingBackend> = mq_ctx;
    let mq_ctx = &mut **mq_ctx;

    mq_ctx.clear(Some((1., 1., 1., 1.)), None, None);
    mq_ctx.begin_default_pass(mq::PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
    mq_ctx.end_render_pass();

    // Run the UI code:
    egui_mq.run(mq_ctx, |_mq_ctx, egui_ctx| {
        let dpi_scale = mq::window::dpi_scale();

        #[retained(default)]
        let ref mut show_egui_demo_windows: bool = true;

        if *show_egui_demo_windows {
            #[retained(default)]
            let ref mut egui_demo_windows: egui_demo_lib::DemoWindows =
                egui_demo_lib::DemoWindows::default();

            egui_demo_windows.ui(egui_ctx);
        }

        // zoom factor could have been changed by the user in egui using Ctrl/Cmd and -/+/0,
        // but it could also be in the middle of being changed by us using the slider. So we
        // only allow egui's zoom to override our zoom if the egui zoom is different from what
        // we saw last time (meaning the user has changed it).
        let curr_egui_zoom = egui_ctx.zoom_factor();

        #[retained(default)]
        let ref mut zoom_factor: f32 = 1.0;
        if *zoom_factor != curr_egui_zoom {
            *zoom_factor = curr_egui_zoom;
        }

        egui::Window::new("egui ❤ miniquad").show(egui_ctx, |ui| {
            egui::widgets::global_dark_light_mode_buttons(ui);
            ui.checkbox(show_egui_demo_windows, "Show egui demo windows");

            ui.group(|ui| {
                ui.label("Physical pixels per each logical 'point':");
                ui.label(format!("native: {:.2}", dpi_scale));
                ui.label(format!("egui:   {:.2}", ui.ctx().pixels_per_point()));
                ui.label("Current zoom factor:");
                ui.add(egui::Slider::new(zoom_factor, 0.75..=3.0).logarithmic(true))
                    .on_hover_text(
                        "Override egui zoom factor manually (changes effective pixels per point)",
                    );
                if ui.button("Reset").clicked() {
                    *zoom_factor = 1.0;
                }

                ui.label("By default, egui allows zooming with\nCtrl/Cmd and +/-/0");
                // Creating a checkbox that directly mutates the egui context's options causes a
                // freeze so we copy the state out, possibly mutate it with the checkbox, and
                // then copy it back in.
                let mut zoom_with_keyboard = egui_ctx.options(|o| o.zoom_with_keyboard);
                ui.checkbox(&mut zoom_with_keyboard, "Allow egui zoom with keyboard");
                egui_ctx.options_mut(|o| o.zoom_with_keyboard = zoom_with_keyboard);
            });

            #[cfg(not(target_arch = "wasm32"))]
            {
                if ui.button("Quit").clicked() {
                    std::process::exit(0);
                }
            }
        });

        // Don't change zoom while dragging the slider
        if !egui_ctx.is_using_pointer() {
            egui_ctx.set_zoom_factor(*zoom_factor);
        }

        egui::Window::new("Color Test").show(egui_ctx, |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    #[retained(default)]
                    let ref mut color_test: egui_demo_lib::ColorTest =
                        egui_demo_lib::ColorTest::default();

                    color_test.ui(ui);
                });
        });
    });

    // Draw things behind egui here
    egui_mq.draw(&mut *mq_ctx);

    // Draw things in front of egui here
    mq_ctx.commit_frame();
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Log to stdout (if you run with `RUST_LOG=debug`).
        tracing_subscriber::fmt::init();
    }

    let conf = mq::conf::Conf {
        high_dpi: true,
        window_width: 1200,
        window_height: 1024,
        ..Default::default()
    };
    mq::start(conf, || Box::new(Stage::new()));
}
