use azteccraft::{
    history::{Command, EditOutcome, History},
    scene::{NodeId, Primitive},
};

pub fn extruded_corner() -> (History, NodeId) {
    let mut history = History::default();
    let EditOutcome::Created(id) = history
        .execute(Command::Create {
            primitive: Primitive::Cube,
            name: "extruded_cube".into(),
        })
        .unwrap()
    else {
        panic!("cube creation failed");
    };
    for face_index in [1, 7] {
        history
            .execute(Command::ExtrudeFace {
                id,
                face_index,
                distance: 0.5,
            })
            .unwrap();
    }
    (history, id)
}
