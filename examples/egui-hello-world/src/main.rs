#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
//! Ported from <https://github.com/emilk/egui/tree/master/examples/hello_world>

use eframe::egui;
use retained::retained;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Box::<App>::default()
        }),
    )
}

#[retained(AppState)]
fn retained_update(ctx: &egui::Context, _frame: &mut eframe::Frame) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("My egui Application");

        #[retained(default)]
        let ref mut name: String = "Arthur".to_string();
        ui.horizontal(|ui| {
            let name_label = ui.label("Your name: ");
            ui.text_edit_singleline(name).labelled_by(name_label.id);
        });

        #[retained(default)]
        let ref mut age: i32 = 0;
        ui.add(egui::Slider::new(age, 0..=120).text("age"));

        if ui.button("Increment").clicked() {
            *age += 1;
        }
        ui.label(format!("Hello '{}', age {}", name, age));

        ui.image(egui::include_image!("../ferris.png"));
    });
}

#[derive(Default)]
struct App(AppState);

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        retained_update(ctx, frame, &mut self.0);
    }
}
