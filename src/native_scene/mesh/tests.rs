use super::*;

#[test]
fn double_and_float_extrusion_preserve_original_bits_and_edge_metadata() {
    for doubles in [false, true] {
        for distance in [0.0, 0.123456789012] {
            let mut mesh = NativeMesh::primitive(Primitive::Cube);
            if doubles {
                let mut points = mesh
                    .points
                    .values()
                    .into_iter()
                    .map(|p| p.to_array())
                    .collect::<Vec<_>>();
                points[0][0] += 1e-10;
                points[0][1] = -0.0;
                points[4][0] -= 1e-10;
                mesh.points = Points::Double3(points);
            } else if let Points::Float3(points) = &mut mesh.points {
                points[0][1] = -0.0;
            }
            mesh.edges[0].smooth = true;
            mesh.edges[5].smooth = true;
            let before = mesh.clone();
            let after = mesh.extruded_face(1, distance).unwrap();
            let old_len = before.points.values().len();
            let prefix = match &after.points {
                Points::Float3(v) => Points::Float3(v[..old_len].into()),
                Points::Double3(v) => Points::Double3(v[..old_len].into()),
            };
            assert_eq!(prefix, before.points);
            assert_eq!(&after.edges[..before.edges.len()], before.edges.as_slice());
            assert_eq!(&after.faces[2..6], &before.faces[2..6]);
            assert_eq!(after.faces[0], before.faces[0]);
            assert_eq!(after.edges.len(), before.edges.len() + 8);
            for i in 0..4 {
                assert_eq!(after.faces[6 + i].outer[0], before.faces[1].outer[i]);
            }
            assert_eq!(matches!(after.points, Points::Double3(_)), doubles);
            if let Points::Double3(points) = &after.points {
                assert_eq!(points[8][0].to_bits(), (-0.5f64 - 1e-10).to_bits());
                assert_eq!(points[8][2].to_bits(), (0.5 + distance).to_bits());
            }
            after.validate().unwrap();
        }
    }
}

#[test]
fn arbitrary_loops_holes_and_reversed_zero_edge_are_retained_without_projection() {
    let mut mesh = NativeMesh::primitive(Primitive::Cube);
    let original_face = mesh.faces[0].clone();
    mesh.faces[0].holes = vec![original_face.outer.clone()];
    mesh.validate().unwrap();
    assert_eq!(mesh.faces[0].holes[0][0].edge, 0);
    let before = mesh.clone();
    assert!(mesh.view(1.0).is_err());
    assert_eq!(
        mesh.extruded_face(0, 1.0),
        Err(NativeError::UnsupportedEdit)
    );
    let after = mesh.extruded_face(1, 1.0).unwrap();
    assert_eq!(after.faces[0], before.faces[0]);
    let zero = DirectedEdge {
        edge: 0,
        reversed: true,
    };
    assert_eq!(
        mesh.endpoints(zero).unwrap(),
        [mesh.edges[0].endpoints[1], mesh.edges[0].endpoints[0]]
    );
    let points = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.5, 2.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let mut ngon = NativeMesh {
        points: Points::Double3(points),
        edges: Vec::new(),
        faces: Vec::new(),
        unsupported_channels: Vec::new(),
    };
    let outer = (0..5).map(|i| ngon.append_edge([i, (i + 1) % 5])).collect();
    ngon.faces.push(NativeFace {
        outer,
        holes: Vec::new(),
    });
    ngon.validate().unwrap();
    assert_eq!(
        ngon.view(1.0),
        Err(NativeError::UnsupportedView("non-quad polygon"))
    );
}

#[test]
fn unsupported_channel_updates_and_new_float_overflow_refuse_without_mutation() {
    for channel in ["crease", "UV", "normal", "color", "tweak", "dependency"] {
        let mut mesh = NativeMesh::primitive(Primitive::Cube);
        mesh.unsupported_channels.push(channel.into());
        let before = mesh.clone();
        assert_eq!(
            mesh.extruded_face(1, 1.0),
            Err(NativeError::UnsupportedEdit)
        );
        assert_eq!(mesh, before);
    }
    let mesh = NativeMesh::primitive(Primitive::Cube);
    assert_eq!(
        mesh.extruded_face(1, f64::from(f32::MAX) * 2.0),
        Err(NativeError::NonFinite)
    );
    let mut mesh = mesh;
    mesh.points = Points::Double3(
        mesh.points
            .values()
            .into_iter()
            .map(|p| p.to_array())
            .collect(),
    );
    let double = mesh.extruded_face(1, f64::from(f32::MAX) * 2.0).unwrap();
    double.validate().unwrap();
    assert!(double.view(1.0).is_err());
    for value in [f64::NAN, f64::INFINITY] {
        if let Points::Double3(points) = &mut mesh.points {
            points[0][0] = value;
        }
        assert_eq!(mesh.validate(), Err(NativeError::NonFinite));
    }
}
