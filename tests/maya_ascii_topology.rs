#[path = "support/maya_ascii_structure/topology.rs"]
mod topology;

use topology::Mesh;

fn cube() -> Mesh {
    Mesh {
        points: vec![
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
            [1.0, 1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ],
        edges: vec![
            [0, 1],
            [1, 2],
            [2, 3],
            [3, 0],
            [4, 5],
            [5, 6],
            [6, 7],
            [7, 4],
            [0, 4],
            [1, 5],
            [2, 6],
            [3, 7],
        ],
        faces: vec![
            vec![-4, -3, -2, -1],
            vec![4, 5, 6, 7],
            vec![0, 9, -5, -9],
            vec![1, 10, -6, -10],
            vec![11, -7, -11, 2],
            vec![8, -8, -12, 3],
        ],
    }
}

fn plane() -> Mesh {
    Mesh {
        points: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
        edges: vec![[0, 1], [1, 2], [2, 3], [3, 0]],
        faces: vec![vec![0, 1, 2, 3]],
    }
}

#[test]
fn hand_traced_cube_has_outward_closed_loops_and_positive_volume() {
    let mesh = cube();
    assert_eq!(
        mesh.loops().unwrap(),
        vec![
            vec![0, 3, 2, 1],
            vec![4, 5, 6, 7],
            vec![0, 1, 5, 4],
            vec![1, 2, 6, 5],
            vec![3, 7, 6, 2],
            vec![0, 4, 7, 3],
        ]
    );
    mesh.check(true).unwrap();
    mesh.check(false).unwrap();
    assert_eq!(mesh.signed_volume().unwrap(), 8.0);
}

#[test]
fn reversing_all_faces_preserves_closure_and_negates_volume() {
    let mut mesh = cube();
    mesh.check(true).unwrap();
    assert_eq!(mesh.signed_volume().unwrap(), 8.0);
    for face in &mut mesh.faces {
        face.reverse();
        for token in face {
            *token = !*token;
        }
    }
    mesh.check(true).unwrap();
    assert_eq!(mesh.loops().unwrap()[0], vec![0, 1, 2, 3]);
    assert_eq!(mesh.signed_volume().unwrap(), -8.0);
}

#[test]
fn hand_traced_plane_obeys_open_boundary_policy() {
    let mesh = plane();
    assert_eq!(mesh.loops().unwrap(), vec![vec![0, 1, 2, 3]]);
    mesh.check(false).unwrap();
    assert_eq!(mesh.signed_volume().unwrap(), 0.0);
    assert!(mesh.check(true).unwrap_err().contains("boundary"));
}

#[test]
fn signed_edge_references_reject_both_out_of_range_directions_and_extremes() {
    for token in [4, -5, i64::MIN, i64::MAX] {
        let mut mesh = plane();
        mesh.check(false).unwrap();
        mesh.faces[0][0] = token;
        assert!(
            mesh.loops().unwrap_err().contains("out of range"),
            "{token}"
        );
        assert!(mesh.signed_volume().is_err(), "{token}");
    }
}

#[test]
fn short_and_discontinuous_faces_are_rejected() {
    for face in [
        vec![],
        vec![0],
        vec![0, 1],
        vec![0, 2, 1, 3],
        vec![0, 1, 2, -4],
    ] {
        let mut mesh = plane();
        mesh.check(false).unwrap();
        mesh.faces[0] = face;
        assert!(mesh.loops().is_err());
        assert!(mesh.check(false).is_err());
    }
}

#[test]
fn repeated_direction_is_rejected_even_for_a_closed_face() {
    let mut mesh = plane();
    mesh.check(false).unwrap();
    mesh.faces.push(mesh.faces[0].clone());
    assert!(mesh.loops().unwrap_err().contains("used twice"));
    assert!(mesh.check(false).is_err());
}

#[test]
fn orphan_duplicate_edge_records_violate_both_boundary_policies() {
    for closed in [false, true] {
        let mut mesh = cube();
        mesh.check(closed).unwrap();
        mesh.edges.push(mesh.edges[0]);
        assert!(mesh.check(closed).unwrap_err().contains("boundary"));
    }
}

#[test]
fn distinct_used_edge_records_can_share_endpoints() {
    let mut mesh = plane();
    mesh.check(false).unwrap();
    mesh.edges.extend_from_within(..);
    mesh.faces.push(vec![4, 5, 6, 7]);
    assert_eq!(
        mesh.loops().unwrap(),
        vec![vec![0, 1, 2, 3], vec![0, 1, 2, 3]]
    );
    assert_eq!(mesh.check(false), Ok(()));
}

#[test]
fn endpoint_indices_are_checked_before_volume_arithmetic() {
    for endpoint in [plane().points.len(), usize::MAX] {
        for side in 0..2 {
            let mut mesh = plane();
            mesh.check(false).unwrap();
            assert_eq!(mesh.signed_volume().unwrap(), 0.0);
            mesh.edges[0][side] = endpoint;
            assert!(mesh.loops().unwrap_err().contains("endpoints"));
            assert!(mesh.signed_volume().unwrap_err().contains("endpoints"));
        }
    }
}

#[test]
fn self_loop_edges_are_rejected() {
    let mut mesh = plane();
    mesh.check(false).unwrap();
    mesh.edges[0] = [0, 0];
    assert!(mesh.loops().unwrap_err().contains("endpoints"));
}

#[test]
fn nonfinite_points_are_rejected_before_volume_arithmetic() {
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for axis in 0..3 {
            let mut mesh = cube();
            mesh.check(true).unwrap();
            assert_eq!(mesh.signed_volume().unwrap(), 8.0);
            mesh.points[0][axis] = value;
            assert!(mesh.loops().unwrap_err().contains("not finite"));
            assert!(mesh.signed_volume().unwrap_err().contains("not finite"));
        }
    }
}
