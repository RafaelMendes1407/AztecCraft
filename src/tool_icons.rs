use eframe::egui::{self, Color32, Painter, Pos2, Rect, Response, Shape, Stroke, Ui};

#[derive(Clone, Copy)]
pub enum Icon {
    Select,
    Cube,
    Plane,
    Move,
    Rotate,
    Scale,
    Frame,
    Undo,
    Redo,
}

pub fn button(ui: &mut Ui, label: &str, icon: Icon, enabled: bool) -> Response {
    tool_button(ui, label, icon, enabled, false)
}

pub fn tool_button(
    ui: &mut Ui,
    label: &str,
    icon: Icon,
    enabled: bool,
    selected: bool,
) -> Response {
    ui.add_enabled_ui(enabled, |ui| {
        let id = ui.make_persistent_id(("tool_icon", label));
        let atom = egui::Atom::custom(id, egui::vec2(20.0, 20.0));
        let output = egui::Button::new((atom, label))
            .selected(selected)
            .gap(6.0)
            .wrap_mode(egui::TextWrapMode::Extend)
            .atom_ui(ui);
        if let Some(rect) = output.rect(id) {
            draw(&ui.painter_at(rect), rect, icon);
        }
        output.response
    })
    .inner
}

pub fn toolbox_width(ui: &Ui) -> f32 {
    let style = ui.style().button_style(
        &Default::default(),
        egui::widget_style::WidgetState::Noninteractive,
    );
    let font = style.text_style.font_id;
    let text = ui.fonts_mut(|fonts| {
        fonts
            .layout_no_wrap("Rotate".into(), font, Color32::WHITE)
            .size()
            .x
    });
    let panel = egui::Frame::side_top_panel(ui.style());
    (text
        + 20.0
        + 6.0
        + style.frame.total_margin().sum().x
        + panel.total_margin().sum().x
        + ui.visuals().widgets.noninteractive.bg_stroke.width)
        .ceil()
}

fn draw(painter: &Painter, rect: Rect, icon: Icon) {
    let point = |x: f32, y: f32| rect.min + egui::vec2(x, y) * (rect.size() / 24.0);
    let amber = Color32::from_rgb(232, 173, 85);
    let cyan = Color32::from_rgb(88, 201, 213);
    let red = Color32::from_rgb(231, 111, 115);
    let green = Color32::from_rgb(141, 200, 123);
    let blue = Color32::from_rgb(116, 169, 235);
    let violet = Color32::from_rgb(185, 154, 221);
    let line = |points: &[(f32, f32)], tint| {
        painter.add(Shape::line(
            points.iter().map(|(x, y)| point(*x, *y)).collect(),
            Stroke::new(1.6, tint),
        ));
    };
    let polygon = |points: &[(f32, f32)], tint: Color32| {
        painter.add(Shape::convex_polygon(
            points.iter().map(|(x, y)| point(*x, *y)).collect(),
            tint.gamma_multiply(0.35),
            Stroke::new(1.4, tint),
        ));
    };
    match icon {
        Icon::Select => {
            for triangle in [
                [(5.0, 3.0), (19.0, 13.0), (12.0, 14.0)],
                [(5.0, 3.0), (12.0, 14.0), (9.0, 21.0)],
            ] {
                painter.add(Shape::convex_polygon(
                    triangle.map(|(x, y)| point(x, y)).to_vec(),
                    blue,
                    Stroke::NONE,
                ));
            }
            painter.add(Shape::closed_line(
                vec![
                    point(5.0, 3.0),
                    point(19.0, 13.0),
                    point(12.0, 14.0),
                    point(9.0, 21.0),
                ],
                Stroke::new(1.0, blue),
            ));
        }
        Icon::Cube => {
            polygon(&[(12.0, 2.0), (21.0, 7.0), (12.0, 12.0), (3.0, 7.0)], amber);
            polygon(
                &[(3.0, 7.0), (12.0, 12.0), (12.0, 22.0), (3.0, 17.0)],
                amber,
            );
            polygon(
                &[(12.0, 12.0), (21.0, 7.0), (21.0, 17.0), (12.0, 22.0)],
                amber,
            );
        }
        Icon::Plane => {
            polygon(
                &[(3.0, 14.0), (10.0, 6.0), (22.0, 10.0), (15.0, 19.0)],
                cyan,
            );
            line(&[(6.5, 10.0), (18.5, 14.5)], cyan);
            line(&[(9.0, 16.5), (16.0, 8.0)], cyan);
        }
        Icon::Move | Icon::Scale => {
            for (end, tint) in [
                ((21.0, 16.0), red),
                ((9.0, 3.0), green),
                ((3.0, 22.0), blue),
            ] {
                line(&[(9.0, 16.0), end], tint);
                match icon {
                    Icon::Scale => {
                        painter.rect_filled(
                            Rect::from_center_size(point(end.0, end.1), rect.size() / 8.0),
                            0.0,
                            tint,
                        );
                    }
                    _ => {
                        let direction = (point(end.0, end.1) - point(9.0, 16.0)).normalized();
                        let tip = point(end.0, end.1);
                        let back = tip - direction * 4.0;
                        painter.add(Shape::line(
                            vec![
                                back + direction.rot90() * 2.5,
                                tip,
                                back - direction.rot90() * 2.5,
                            ],
                            Stroke::new(1.6, tint),
                        ));
                    }
                }
            }
        }
        Icon::Rotate => {
            painter.circle_stroke(rect.center(), rect.width() * 0.38, Stroke::new(1.5, blue));
            line(
                &[
                    (4.0, 16.0),
                    (4.0, 8.0),
                    (8.0, 4.0),
                    (16.0, 4.0),
                    (20.0, 8.0),
                ],
                green,
            );
            line(
                &[
                    (8.0, 3.0),
                    (13.0, 7.0),
                    (15.0, 12.0),
                    (13.0, 17.0),
                    (8.0, 21.0),
                ],
                red,
            );
        }
        Icon::Frame => {
            for points in [
                [(3.0, 9.0), (3.0, 3.0), (9.0, 3.0)],
                [(15.0, 3.0), (21.0, 3.0), (21.0, 9.0)],
                [(21.0, 15.0), (21.0, 21.0), (15.0, 21.0)],
                [(9.0, 21.0), (3.0, 21.0), (3.0, 15.0)],
            ] {
                line(&points, violet);
            }
            painter.circle_filled(rect.center(), 2.2, blue);
        }
        Icon::Undo | Icon::Redo => {
            let redo = matches!(icon, Icon::Redo);
            let mirror = |p: Pos2| {
                if redo {
                    Pos2::new(rect.center().x * 2.0 - p.x, p.y)
                } else {
                    p
                }
            };
            let tint = if redo { violet } else { blue };
            let points = (0..=16)
                .map(|i| {
                    let angle = -2.6 + i as f32 / 16.0 * 4.2;
                    mirror(point(12.0 + angle.cos() * 8.0, 12.0 + angle.sin() * 8.0))
                })
                .collect();
            painter.add(Shape::line(points, Stroke::new(1.8, tint)));
            painter.add(Shape::convex_polygon(
                vec![
                    mirror(point(3.0, 3.0)),
                    mirror(point(3.0, 11.0)),
                    mirror(point(10.0, 8.0)),
                ],
                tint,
                Stroke::NONE,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolbox_buttons_fit_on_one_line_at_minimum_window_and_enlarged_text() {
        for font_size in [14.0, 18.0] {
            let ctx = egui::Context::default();
            let mut style = egui::Style {
                override_font_id: Some(egui::FontId::proportional(font_size)),
                ..Default::default()
            };
            style.spacing.button_padding = egui::vec2(8.0, 4.0);
            ctx.set_style_of(egui::Theme::Dark, style);
            ctx.set_theme(egui::Theme::Dark);
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(960.0, 640.0))),
                    ..Default::default()
                },
                |ui| {
                    egui::Panel::left("tools")
                        .exact_size(toolbox_width(ui))
                        .resizable(false)
                        .show(ui, |ui| {
                            let available = ui.max_rect();
                            let movement = button(ui, "Move", Icon::Move, false);
                            let rotation = button(ui, "Rotate", Icon::Rotate, false);
                            assert_eq!(movement.rect.height(), rotation.rect.height());
                            assert!(
                                rotation.rect.right() <= available.right(),
                                "{font_size}: {:?} outside {available:?}",
                                rotation.rect
                            );
                            assert!(!rotation.enabled());
                        });
                },
            );
            output.textures_delta.clear();
        }
    }
}
