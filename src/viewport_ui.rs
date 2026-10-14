use azteccraft::{
    scene::{NodeId, Scene},
    viewport::{Camera, InvalidCameraInput},
};
use eframe::egui::{self, Color32, PointerButton, Pos2, Rect, Shape, Stroke};
use glam::{Vec2, Vec3};

pub struct ViewportOutput {
    pub selection: Option<Option<NodeId>>,
    pub aspect: f32,
}

pub fn show(
    ui: &mut egui::Ui,
    camera: &mut Camera,
    scene: &Scene,
    selected: Option<NodeId>,
) -> Result<ViewportOutput, InvalidCameraInput> {
    let (response, painter) =
        ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
    let rect = response.rect;
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return Ok(ViewportOutput {
            selection: None,
            aspect: 1.0,
        });
    }
    let aspect = rect.width() / rect.height();
    painter.rect_filled(rect, 0.0, Color32::from_gray(31));
    let (delta, alt, scroll) =
        ui.input(|i| (i.pointer.delta(), i.modifiers.alt, i.smooth_scroll_delta.y));
    if response.dragged_by(PointerButton::Primary) && alt {
        camera.orbit(Vec2::new(-delta.x, delta.y) * 0.007)?;
    }
    if response.dragged_by(PointerButton::Middle) {
        camera.pan(Vec2::new(-delta.x, delta.y) * camera.distance() * 0.002)?;
    }
    if response.dragged_by(PointerButton::Secondary) {
        camera.dolly((delta.y * 0.01).exp())?;
    }
    if response.hovered() && scroll != 0.0 {
        camera.dolly((-scroll * 0.002).exp())?;
    }
    let mut selection = None;
    if response.clicked_by(PointerButton::Primary)
        && !alt
        && let Some(point) = response.interact_pointer_pos()
    {
        let p = (point - rect.min) / rect.size();
        selection =
            Some(camera.pick(scene, Vec2::new(p.x * 2.0 - 1.0, 1.0 - p.y * 2.0), aspect)?);
    }
    for i in -10..=10 {
        let coordinate = i as f32;
        let color = if i == 0 {
            Color32::from_gray(85)
        } else {
            Color32::from_gray(48)
        };
        for (a, b) in [
            (
                Vec3::new(coordinate, 0.0, -10.0),
                Vec3::new(coordinate, 0.0, 10.0),
            ),
            (
                Vec3::new(-10.0, 0.0, coordinate),
                Vec3::new(10.0, 0.0, coordinate),
            ),
        ] {
            if let (Some(a), Some(b)) = (
                project(camera, a, rect, aspect)?,
                project(camera, b, rect, aspect)?,
            ) {
                painter.line_segment([a, b], Stroke::new(1.0, color));
            }
        }
    }
    let mut faces = Vec::new();
    for node in scene.nodes() {
        let vertices = node.world_vertices();
        for face in node.mesh().faces() {
            let mut points = Vec::new();
            let mut depth = 0.0;
            for index in face {
                let world = vertices[*index];
                if let Some(point) = project(camera, world, rect, aspect)? {
                    points.push(point);
                }
                depth += -camera.view_matrix().transform_point3(world).z / 4.0;
            }
            if points.len() == 4 && depth.is_finite() {
                faces.push((depth, points, selected == Some(node.id())));
            }
        }
    }
    faces.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (_, points, is_selected) in faces {
        let fill = if is_selected {
            Color32::from_rgb(95, 110, 120)
        } else {
            Color32::from_gray(83)
        };
        let edge = if is_selected {
            Color32::from_rgb(242, 163, 70)
        } else {
            Color32::from_gray(145)
        };
        painter.add(Shape::convex_polygon(points, fill, Stroke::new(1.2, edge)));
    }
    painter.text(
        rect.left_bottom() + egui::vec2(12.0, -12.0),
        egui::Align2::LEFT_BOTTOM,
        format!("{} objects | Perspective", scene.nodes().len()),
        egui::FontId::proportional(12.0),
        Color32::GRAY,
    );
    Ok(ViewportOutput { selection, aspect })
}

fn project(
    camera: &Camera,
    world: Vec3,
    rect: Rect,
    aspect: f32,
) -> Result<Option<Pos2>, InvalidCameraInput> {
    let Some(ndc) = camera.project(world, aspect)? else {
        return Ok(None);
    };
    let x = rect.min.x + (ndc.x + 1.0) * 0.5 * rect.width();
    let y = rect.min.y + (1.0 - ndc.y) * 0.5 * rect.height();
    if !x.is_finite() || !y.is_finite() {
        return Err(InvalidCameraInput);
    }
    // Bound lateral coordinates before tessellation; the painter clips to the viewport.
    Ok(Some(Pos2::new(
        x.clamp(rect.min.x - 100_000.0, rect.max.x + 100_000.0),
        y.clamp(rect.min.y - 100_000.0, rect.max.y + 100_000.0),
    )))
}
