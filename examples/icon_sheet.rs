//! Dev-only gallery of every UI icon: `cargo run --example icon_sheet -- --disabled --zoom 2`.

use azteccraft::ui_icons;
use eframe::egui;

struct Sheet {
    disabled: bool,
    zoom: Option<f32>,
}

impl eframe::App for Sheet {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(zoom) = self.zoom {
            ui.ctx().set_pixels_per_point(zoom);
        }
        egui::ScrollArea::both().show(ui, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(43, 43, 43))
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.label(format!("disabled: {}", self.disabled));
                    ui.add_enabled_ui(!self.disabled, |ui| {
                        for icon in ui_icons::ALL {
                            ui.horizontal(|ui| {
                                ui.label(format!("{icon:?}"));
                                for size in [16.0, 24.0, 32.0] {
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(size, size),
                                        egui::Sense::hover(),
                                    );
                                    ui_icons::paint(&ui.painter_at(rect), rect, *icon);
                                    ui.label(format!("{size}px"));
                                }
                            });
                        }
                    });
                });
        });
    }
}

fn parse_args() -> Result<(bool, Option<f32>), String> {
    let mut args = std::env::args().skip(1);
    let mut disabled = false;
    let mut zoom = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--disabled" => disabled = true,
            "--zoom" => {
                let value = args.next().and_then(|v| v.parse::<f32>().ok());
                match value {
                    Some(z) if z.is_finite() && z > 0.0 && z <= 8.0 => zoom = Some(z),
                    _ => return Err("--zoom expects a number in (0, 8]".to_string()),
                }
            }
            _ => return Err(format!("unknown argument {arg}")),
        }
    }
    Ok((disabled, zoom))
}

fn main() -> eframe::Result {
    let (disabled, zoom) = match parse_args() {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };
    eframe::run_native(
        "icon_sheet",
        eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default().with_inner_size([960.0, 900.0]),
            ..Default::default()
        },
        Box::new(move |_cc| Ok(Box::new(Sheet { disabled, zoom }))),
    )
}
