#![forbid(unsafe_code)]

mod app;
mod viewport_ui;

fn main() -> eframe::Result {
    eframe::run_native(
        "AztecCraft | Untitled",
        eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_inner_size([1440.0, 900.0])
                .with_min_inner_size([960.0, 640.0]),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(app::Editor::new(cc)))),
    )
}
