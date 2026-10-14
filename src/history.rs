use crate::scene::{NodeId, Primitive, Scene, SceneError, Transform};

#[derive(Clone, Debug)]
pub enum Command {
    Create { primitive: Primitive, name: String },
    Remove(NodeId),
    Rename { id: NodeId, name: String },
    SetTransform { id: NodeId, transform: Transform },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditOutcome {
    Created(NodeId),
    Changed,
    Unchanged,
}

#[derive(Default)]
pub struct History {
    scene: Scene,
    undo: Vec<Scene>,
    redo: Vec<Scene>,
}

impl History {
    pub fn new(scene: Scene) -> Self {
        Self {
            scene,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn execute(&mut self, command: Command) -> Result<EditOutcome, SceneError> {
        let before = self.scene.clone();
        let outcome = match command {
            Command::Create { primitive, name } => {
                EditOutcome::Created(self.scene.create(primitive, &name)?)
            }
            Command::Remove(id) => {
                self.scene.remove(id)?;
                EditOutcome::Changed
            }
            Command::Rename { id, name } => {
                self.scene.rename(id, &name)?;
                EditOutcome::Changed
            }
            Command::SetTransform { id, transform } => {
                self.scene.set_transform(id, transform)?;
                EditOutcome::Changed
            }
        };
        if self.scene == before {
            return Ok(EditOutcome::Unchanged);
        }
        self.undo.push(before);
        self.redo.clear();
        Ok(outcome)
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo.pop() else {
            return false;
        };
        self.redo.push(self.scene.clone());
        self.scene.restore_authoring(previous);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(self.scene.clone());
        self.scene.restore_authoring(next);
        true
    }
}
