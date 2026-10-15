use crate::editor_operations::Operation;
use azteccraft::{
    native_scene::NativeId,
    view_scene::ViewScene,
    viewport::{Camera, InvalidCameraInput},
};
use eframe::egui::{self, Color32, Painter, PointerButton, Pos2, Rect, Shape, Stroke};
use glam::{Vec2, Vec3};

pub struct ViewportInput {
    painter: Painter,
    rect: Rect,
    click_ndc: Option<Vec2>,
    navigation: Vec<Operation>,
}

pub fn capture(
    ui: &mut egui::Ui,
    camera: &Camera,
) -> Result<Option<ViewportInput>, InvalidCameraInput> {
    let (response, painter) =
        ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
    let rect = response.rect.intersect(ui.clip_rect());
    if !rect.is_finite() {
        return Err(InvalidCameraInput);
    }
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return Ok(None);
    }
    let aspect = rect.width() / rect.height();
    if !aspect.is_finite() || aspect <= 0.0 {
        return Err(InvalidCameraInput);
    }
    let painter = painter.with_clip_rect(rect);
    let (delta, alt, scroll) =
        ui.input(|i| (i.pointer.delta(), i.modifiers.alt, i.smooth_scroll_delta.y));
    let mut navigation = Vec::with_capacity(4);
    if response.dragged_by(PointerButton::Primary) && alt {
        navigation.push(Operation::Orbit {
            radians: (Vec2::new(-delta.x, delta.y) * 0.007).to_array(),
        });
    }
    if response.dragged_by(PointerButton::Middle) {
        navigation.push(Operation::Pan {
            world_units: (Vec2::new(-delta.x, delta.y) * camera.distance() * 0.002).to_array(),
        });
    }
    if response.dragged_by(PointerButton::Secondary) {
        navigation.push(Operation::Dolly {
            factor: (delta.y * 0.01).exp(),
        });
    }
    if response.hovered() && scroll != 0.0 {
        navigation.push(Operation::Dolly {
            factor: (-scroll * 0.002).exp(),
        });
    }
    let mut click_ndc = None;
    if response.clicked_by(PointerButton::Primary)
        && !alt
        && let Some(point) = response.interact_pointer_pos()
    {
        let p = (point - rect.min) / rect.size();
        let ndc = Vec2::new(p.x * 2.0 - 1.0, 1.0 - p.y * 2.0);
        if !ndc.is_finite() {
            return Err(InvalidCameraInput);
        }
        click_ndc = Some(ndc);
    }
    Ok(Some(ViewportInput {
        painter,
        rect,
        click_ndc,
        navigation,
    }))
}

impl ViewportInput {
    pub fn aspect(&self) -> f32 {
        self.rect.width() / self.rect.height()
    }
    pub fn click_ndc(&self) -> Option<Vec2> {
        self.click_ndc
    }
    pub fn take_navigation(&mut self) -> Vec<Operation> {
        std::mem::take(&mut self.navigation)
    }

    pub fn paint(
        &self,
        camera: &Camera,
        scene: &ViewScene,
        selected_prefix: Option<&[NativeId]>,
    ) -> Result<(), InvalidCameraInput> {
        let painter = &self.painter;
        let rect = self.rect;
        let aspect = self.aspect();
        painter.rect_filled(rect, 0.0, Color32::from_gray(31));
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
        for instance in scene.instances() {
            let vertices = instance.world_vertices();
            for face in instance.faces() {
                let corners = face.corners();
                let mut points = Vec::with_capacity(corners.len());
                let mut depth = 0.0;
                for index in corners {
                    let world = vertices[*index];
                    if let Some(point) = project(camera, world, rect, aspect)? {
                        points.push(point);
                    }
                    depth += -camera.view_matrix().transform_point3(world).z / corners.len() as f32;
                }
                if points.len() == corners.len() && depth.is_finite() {
                    faces.push((
                        depth,
                        points,
                        selected_prefix.is_some_and(|path| instance.path().starts_with(path)),
                        face,
                    ));
                }
            }
        }
        faces.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (_, points, is_selected, face) in faces {
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
            let mut mesh = egui::Mesh::default();
            for point in &points {
                mesh.colored_vertex(*point, fill);
            }
            for triangle in face.triangles() {
                let [a, b, c] = triangle.map(|index| {
                    face.corners()
                        .iter()
                        .position(|corner| *corner == index)
                        .expect("view triangles use source face corners") as u32
                });
                mesh.add_triangle(a, b, c);
            }
            painter.add(Shape::mesh(mesh));
            // Filled path miter/feather joins can explode on acute projected source faces.
            for index in 0..points.len() {
                painter.line_segment(
                    [points[index], points[(index + 1) % points.len()]],
                    Stroke::new(1.2, edge),
                );
            }
        }
        let appearance = if scene
            .instances()
            .iter()
            .any(|instance| instance.appearance_incomplete())
        {
            "\nAppearance incomplete: geometry preview"
        } else {
            ""
        };
        painter.text(
            rect.left_bottom() + egui::vec2(12.0, -12.0),
            egui::Align2::LEFT_BOTTOM,
            format!(
                "{} instances | Perspective\nCPU preview: depth/clipping incomplete{appearance}",
                scene.instances().len()
            ),
            egui::FontId::proportional(12.0),
            Color32::GRAY,
        );
        Ok(())
    }
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
