use super::*;
use azteccraft::{
    native_scene::{NativeScene, NativeTransform},
    scene::Primitive,
};
use egui::{Event, Modifiers, MouseWheelUnit};

mod maya_binary {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/maya_binary.rs"
    ));
}

fn camera() -> Camera {
    Camera::new(Vec3::ZERO, 0.0, 0.0, 5.0, 45_f32.to_radians(), 0.01, 100.0).unwrap()
}

fn raw(time: f64, modifiers: Modifiers, mut events: Vec<Event>) -> egui::RawInput {
    events.insert(0, Event::ModifiersChanged(modifiers));
    egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(128.0, 96.0))),
        time: Some(time),
        events,
        ..Default::default()
    }
}

fn capture_frame(
    context: &egui::Context,
    camera: &Camera,
    raw: egui::RawInput,
    clip: Rect,
) -> (ViewportInput, f32) {
    let mut captured = None;
    let mut scroll = 0.0;
    let mut output = context.run_ui(raw, |ui| {
        ui.set_clip_rect(clip);
        scroll = ui.input(|input| input.smooth_scroll_delta.y);
        captured = capture(ui, camera).unwrap();
    });
    output.textures_delta.clear();
    (captured.unwrap(), scroll)
}

#[test]
fn clipped_effective_rect_drives_click_ndc_without_mutating_camera() {
    let context = egui::Context::default();
    let camera = camera();
    let before = camera.clone();
    let clip = Rect::from_min_max(egui::pos2(16.0, 8.0), egui::pos2(112.0, 88.0));
    let point = egui::pos2(88.0, 28.0);
    let _ = capture_frame(
        &context,
        &camera,
        raw(0.0, Modifiers::NONE, vec![Event::PointerMoved(point)]),
        clip,
    );
    let press = Event::PointerButton {
        pos: point,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    };
    let _ = capture_frame(
        &context,
        &camera,
        raw(
            1.0 / 60.0,
            Modifiers::NONE,
            vec![Event::PointerMoved(point), press],
        ),
        clip,
    );
    let release = Event::PointerButton {
        pos: point,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    };
    let (mut input, _) = capture_frame(
        &context,
        &camera,
        raw(2.0 / 60.0, Modifiers::NONE, vec![release]),
        clip,
    );
    assert_eq!(input.rect, clip);
    assert_eq!(input.aspect(), 1.2);
    assert_eq!(input.click_ndc(), Some(Vec2::splat(0.5)));
    assert!(input.take_navigation().is_empty());
    assert_eq!(camera, before);
}

#[test]
fn captured_typed_navigation_preserves_orbit_pan_and_both_dollies_and_drains_once() {
    let context = egui::Context::default();
    let mut camera = camera();
    camera
        .frame_points(&[Vec3::splat(-4.0), Vec3::splat(4.0)], 1.0)
        .unwrap();
    let before = camera.clone();
    let modifiers = Modifiers {
        alt: true,
        ..Modifiers::NONE
    };
    let clip = Rect::from_min_size(Pos2::ZERO, egui::vec2(128.0, 96.0));
    let point = egui::pos2(40.0, 30.0);
    let _ = capture_frame(
        &context,
        &camera,
        raw(0.0, modifiers, vec![Event::PointerMoved(point)]),
        clip,
    );
    let mut events = vec![Event::PointerMoved(point)];
    for button in [
        PointerButton::Primary,
        PointerButton::Middle,
        PointerButton::Secondary,
    ] {
        events.push(Event::PointerButton {
            pos: point,
            button,
            pressed: true,
            modifiers,
        });
    }
    let _ = capture_frame(&context, &camera, raw(1.0 / 60.0, modifiers, events), clip);
    let (mut input, scroll) = capture_frame(
        &context,
        &camera,
        raw(
            2.0 / 60.0,
            modifiers,
            vec![
                Event::PointerMoved(egui::pos2(50.0, 50.0)),
                Event::MouseWheel {
                    unit: MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, 20.0),
                    phase: egui::TouchPhase::Move,
                    modifiers,
                },
            ],
        ),
        clip,
    );
    assert_ne!(scroll, 0.0);
    let operations = input.take_navigation();
    let [
        Operation::Orbit { radians },
        Operation::Pan { world_units },
        Operation::Dolly { factor: secondary },
        Operation::Dolly { factor: wheel },
    ] = operations.as_slice()
    else {
        panic!("captured operations must keep the existing order")
    };
    assert_eq!(*radians, (Vec2::new(-10.0, 20.0) * 0.007).to_array());
    assert_eq!(
        *world_units,
        (Vec2::new(-10.0, 20.0) * camera.distance() * 0.002).to_array()
    );
    assert_eq!(*secondary, 0.2_f32.exp());
    assert_eq!(*wheel, (-scroll * 0.002).exp());
    assert!(input.take_navigation().is_empty());
    assert_eq!(camera, before);
}

#[test]
fn canonical_quad_instances_paint_only_the_selected_prefix_and_persistent_limitations() {
    let mut scene = NativeScene::default();
    for (name, x) in [("first", -1.0), ("second", 1.0)] {
        let id = scene.create(Primitive::Cube, name).unwrap();
        scene
            .set_transform(
                id,
                NativeTransform {
                    translation: [x, 0.0, 0.0],
                    ..Default::default()
                },
            )
            .unwrap();
    }
    let before = scene.clone();
    let view = ViewScene::from_native(&scene, 256 * 1024 * 1024).unwrap();
    let path = view.instances()[0].path();
    let selected = &path[..path.len() - 1];
    let camera = camera();
    let context = egui::Context::default();
    let mut output = context.run_ui(raw(0.0, Modifiers::NONE, Vec::new()), |ui| {
        capture(ui, &camera)
            .unwrap()
            .unwrap()
            .paint(&camera, &view, Some(selected))
            .unwrap();
    });
    let selected_faces = output.shapes.iter().filter(|shape| matches!(&shape.shape, Shape::Mesh(mesh) if mesh.vertices.len() == 4 && mesh.indices.len() == 6 && mesh.vertices.iter().all(|vertex| vertex.color == Color32::from_rgb(95, 110, 120)))).count();
    let dormant_faces = output.shapes.iter().filter(|shape| matches!(&shape.shape, Shape::Mesh(mesh) if mesh.vertices.len() == 4 && mesh.indices.len() == 6 && mesh.vertices.iter().all(|vertex| vertex.color == Color32::from_gray(83)))).count();
    assert_eq!((selected_faces, dormant_faces), (6, 6));
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, Shape::Text(text) if text.galley.job.text.contains("CPU preview: depth/clipping incomplete"))));
    output.textures_delta.clear();
    assert_eq!(scene, before);
}

#[test]
fn actual_binary_triangle_paints_source_corners_and_persistent_appearance_limit() {
    let mut scene =
        azteccraft::native_scene::binary::decode(maya_binary::specimen().into()).unwrap();
    let node = scene.nodes()[0].id();
    scene
        .set_channel(node, azteccraft::native_scene::Channel::TranslateX, 0.0)
        .unwrap();
    let before = scene.clone();
    let view = ViewScene::from_native(&scene, 1024 * 1024).unwrap();
    assert_eq!(view.instances()[0].faces()[0].corners().len(), 3);
    let path = view.instances()[0].path();
    let mut camera = camera();
    camera.frame_view(&view, None, 4.0 / 3.0).unwrap();
    let context = egui::Context::default();
    let mut output = context.run_ui(raw(0.0, Modifiers::NONE, Vec::new()), |ui| {
        capture(ui, &camera)
            .unwrap()
            .unwrap()
            .paint(&camera, &view, Some(&path[..path.len() - 1]))
            .unwrap();
    });
    let meshes = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            Shape::Mesh(mesh)
                if mesh
                    .vertices
                    .iter()
                    .all(|vertex| vertex.color == Color32::from_rgb(95, 110, 120)) =>
            {
                Some(mesh)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(meshes.len(), 1);
    assert_eq!(meshes[0].vertices.len(), 3);
    assert_eq!(meshes[0].indices, [0, 1, 2]);
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, Shape::Text(text) if text.galley.job.text.contains("Appearance incomplete: geometry preview"))));
    output.textures_delta.clear();
    assert_eq!(scene, before);
}

#[test]
fn actual_binary_group_prefix_highlights_child_geometry_and_excludes_outside_geometry() {
    use maya_binary::{chunk, creation, group, mesh_payload, scene, triple};
    let children = [
        group(b"XFRM", &creation(b"assembly", None), 4),
        group(
            b"XFRM",
            &[
                creation(b"part", Some(b"assembly")),
                triple(b"t", [-1.0, 0.0, 0.0]),
            ]
            .concat(),
            4,
        ),
        group(
            b"DMSH",
            &[
                creation(b"partShape", Some(b"part")),
                chunk(b"MESH", &mesh_payload(&[0, 1, 0x6000_0002]), 8),
            ]
            .concat(),
            4,
        ),
        group(
            b"XFRM",
            &[creation(b"outside", None), triple(b"t", [1.0, 0.0, 0.0])].concat(),
            4,
        ),
        group(
            b"DMSH",
            &[
                creation(b"outsideShape", Some(b"outside")),
                chunk(b"MESH", &mesh_payload(&[0, 1, 0x6000_0002]), 8),
            ]
            .concat(),
            4,
        ),
    ]
    .concat();
    let scene = azteccraft::native_scene::binary::decode(scene(&children)).unwrap();
    let before = scene.clone();
    let prefix = [scene
        .nodes()
        .iter()
        .find(|node| node.name() == "assembly")
        .unwrap()
        .transform_id()];
    let view = ViewScene::from_native(&scene, 1024 * 1024).unwrap();
    assert_eq!(view.instances().len(), 2);
    let mut camera = camera();
    camera.frame_view(&view, None, 4.0 / 3.0).unwrap();
    let context = egui::Context::default();
    let mut output = context.run_ui(raw(0.0, Modifiers::NONE, Vec::new()), |ui| {
        capture(ui, &camera)
            .unwrap()
            .unwrap()
            .paint(&camera, &view, Some(&prefix))
            .unwrap();
    });
    for (color, expected) in [
        (Color32::from_rgb(95, 110, 120), 1),
        (Color32::from_gray(83), 1),
    ] {
        let count = output.shapes.iter().filter(|shape| matches!(&shape.shape, Shape::Mesh(mesh) if mesh.vertices.len() == 3 && mesh.vertices.iter().all(|vertex| vertex.color == color))).count();
        assert_eq!(count, expected);
    }
    output.textures_delta.clear();
    assert_eq!(scene, before);
}

#[test]
fn acute_native_face_tessellation_stays_near_its_bounded_projected_geometry() {
    let mut legacy = azteccraft::scene::Scene::default();
    legacy.create(Primitive::Plane, "acute").unwrap();
    let mut json = serde_json::to_value(legacy).unwrap();
    json["nodes"][0]["mesh"]["vertices"] = serde_json::json!([
        [-0.5, 0.0, 0.0],
        [0.5, 0.0, 0.0],
        [-0.4999, 0.0001, 0.0],
        [-0.5, 0.0001, 0.0]
    ]);
    let native = NativeScene::from_legacy(serde_json::from_value(json).unwrap()).unwrap();
    let view = ViewScene::from_native(&native, 1024 * 1024).unwrap();
    let path = view.instances()[0].path();
    let camera = camera();
    for vertex in view.instances()[0].world_vertices() {
        assert!(
            camera
                .project(*vertex, 4.0 / 3.0)
                .unwrap()
                .unwrap()
                .truncate()
                .abs()
                .max_element()
                < 1.0
        );
    }
    let context = egui::Context::default();
    let mut output = context.run_ui(raw(0.0, Modifiers::NONE, Vec::new()), |ui| {
        capture(ui, &camera)
            .unwrap()
            .unwrap()
            .paint(&camera, &view, Some(&path[..path.len() - 1]))
            .unwrap();
    });
    output.textures_delta.clear();
    let fill = Color32::from_rgb(95, 110, 120);
    let edge = Color32::from_rgb(242, 163, 70);
    let face_shapes = output
        .shapes
        .into_iter()
        .filter(|shape| match &shape.shape {
            Shape::Path(path) => path.fill == fill,
            Shape::Mesh(mesh) => mesh.vertices.iter().any(|vertex| vertex.color == fill),
            Shape::LineSegment { stroke, .. } => stroke.color == edge,
            _ => false,
        })
        .collect::<Vec<_>>();
    assert!(!face_shapes.is_empty());
    let jobs = context.tessellate(face_shapes, output.pixels_per_point);
    let bound = Rect::from_min_size(Pos2::ZERO, egui::vec2(128.0, 96.0)).expand(4.0);
    for job in jobs {
        if let egui::epaint::Primitive::Mesh(mesh) = job.primitive {
            for vertex in mesh.vertices {
                assert!(
                    vertex.pos.is_finite() && bound.contains(vertex.pos),
                    "unexpected tessellated face vertex {:?}",
                    vertex.pos
                );
            }
        }
    }
}

#[test]
#[ignore = "requires an explicitly supplied private Maya binary fixture"]
fn private_native_open_face_tessellation_preserves_prepared_frustum_bounds() {
    let path =
        std::env::var("AZTECCRAFT_PRIVATE_MB").expect("set the private fixture path explicitly");
    let document = azteccraft::document::load_native(std::path::Path::new(&path)).unwrap();
    let view = ViewScene::from_native(&document.scene, 256 * 1024 * 1024).unwrap();
    let mut camera = Camera::default();
    camera.frame_view(&view, None, 4.0 / 3.0).unwrap();
    for vertex in view
        .instances()
        .iter()
        .flat_map(|instance| instance.world_vertices())
    {
        let ndc = camera.project(*vertex, 4.0 / 3.0).unwrap().unwrap();
        assert!(ndc.truncate().abs().max_element() <= 1.0);
    }
    let context = egui::Context::default();
    let mut output = context.run_ui(raw(0.0, Modifiers::NONE, Vec::new()), |ui| {
        capture(ui, &camera)
            .unwrap()
            .unwrap()
            .paint(&camera, &view, None)
            .unwrap();
    });
    output.textures_delta.clear();
    let bound = Rect::from_min_size(Pos2::ZERO, egui::vec2(128.0, 96.0)).expand(4.0);
    let mut faces = 0;
    for shape in output.shapes {
        let is_face = match &shape.shape {
            Shape::Path(path) => path.fill == Color32::from_gray(83),
            Shape::Mesh(mesh) => mesh
                .vertices
                .iter()
                .any(|vertex| vertex.color == Color32::from_gray(83)),
            Shape::LineSegment { stroke, .. } => stroke.color == Color32::from_gray(145),
            _ => false,
        };
        if !is_face {
            continue;
        }
        faces += 1;
        let description = format!("{:?}", shape.shape);
        for job in context.tessellate(vec![shape], output.pixels_per_point) {
            if let egui::epaint::Primitive::Mesh(mesh) = job.primitive {
                for vertex in mesh.vertices {
                    assert!(
                        vertex.pos.is_finite() && bound.contains(vertex.pos),
                        "face primitive {description}; unexpected tessellated vertex {:?}",
                        vertex.pos
                    );
                }
            }
        }
    }
    assert!(faces > 0);
}
