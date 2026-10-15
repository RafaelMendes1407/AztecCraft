//! Original vector UI icons, drawn in code on a 24-unit grid. Each glyph is
//! authored from a written spec; nothing is traced, copied or derived from any Maya asset.

use eframe::egui::{self, Color32, Pos2, Rect, Shape, Stroke, epaint::PathShape};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiIcon {
    Select,
}

pub const ALL: &[UiIcon] = &[UiIcon::Select];

// Newly chosen values; not sampled from any Maya image.
const NEUTRAL_LIGHT: Color32 = Color32::from_rgb(214, 218, 222);

const GRID: f32 = 24.0;

/// One closed, fan-filled path per glyph. egui fills a closed path as a fan from
/// vertex 0, so outlines are clockwise (y down) and star-shaped from vertex 0.
/// Overlapping pieces are avoided: they double-composite under disabled opacity.
fn outline(rect: Rect, grid_points: &[(f32, f32)], color: Color32) -> Shape {
    let scale = rect.width().min(rect.height()) / GRID;
    let points = grid_points
        .iter()
        .map(|&(x, y)| Pos2::new(rect.min.x + x * scale, rect.min.y + y * scale))
        .collect();
    let mut path = PathShape::closed_line(points, Stroke::NONE);
    path.fill = color;
    Shape::Path(path)
}

/// Geometry of `icon` fitted to `rect`, without needing a window. Disabled looks
/// come from the host `Ui` (egui's disabled opacity); icons have no tone.
pub fn shapes(icon: UiIcon, rect: Rect) -> Vec<Shape> {
    let color = NEUTRAL_LIGHT;
    match icon {
        // Solid arrow pointer, tip upper-left, angled down-right, short tail.
        UiIcon::Select => vec![outline(
            rect,
            &[
                (2.5, 2.0),   // tip
                (22.0, 13.5), // wing
                (10.5, 13.5), // notch
                (13.5, 20.5), // tail right
                (12.0, 23.0), // tail end
                (7.5, 14.5),  // tail left
                (2.5, 19.5),  // heel
            ],
            color,
        )],
    }
}

pub fn paint(painter: &egui::Painter, rect: Rect, icon: UiIcon) {
    painter.extend(shapes(icon, rect));
}

#[cfg(test)]
mod tests;
